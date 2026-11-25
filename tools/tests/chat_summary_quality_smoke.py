#!/usr/bin/env python3
"""Exercise the exported production summary prompt with a live DeepSeek model and synthetic history."""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import urllib.error
import urllib.request


def load_provider(preferences):
    """Read the configured summary binding and credentials without exporting their values."""
    models = json.loads((preferences / "model_configs.preferences.json").read_text(encoding="utf-8-sig"))
    functions = json.loads((preferences / "functional_configs.preferences.json").read_text(encoding="utf-8-sig"))
    binding = functions["function_model_binding"]["SUMMARY"]
    provider = models["provider_" + binding["providerId"]]
    if provider["providerType"] != "DEEPSEEK" or not provider["endpoint"].endswith("/chat/completions"):
        raise ValueError("This quality smoke runner requires a configured DeepSeek Chat Completions summary binding")
    if provider["useMultipleApiKeys"]:
        raise ValueError("Use a single-key test profile; the runner does not implement the application's key-pool policy")
    model = next(item for item in provider["models"] if item["id"] == binding["modelId"])
    return provider, model, binding


def inspect_summary(text, case, finish_reason, tool_calls):
    """Check fixed structure and required facts; semantic correctness is reviewed separately."""
    headers = ["==========对话摘要==========", "【核心任务状态】", "【互动情节与设定】",
               "【对话历程与概要】", "【关键信息与上下文】", "============================"]
    positions = [text.find(header) for header in headers]
    checks = {
        "fixed_structure": all(index >= 0 for index in positions) and positions == sorted(positions),
        "summary_only": text.startswith(headers[0]) and text.endswith(headers[-1]),
        "completed_without_tools": finish_reason == "stop" and not tool_calls,
    }
    for check in case["checks"]:
        checks[check["id"]] = (all(term in text for term in check.get("all", []))
                              and ("any" not in check or any(term in text for term in check["any"])))
    return checks


def run_case(provider, model, case, tools, prompt, output, repeat):
    """Send unchanged synthetic conversation messages plus the exact production directive over SSE."""
    body = {"model": model["id"], "messages": [*case["messages"], {"role": "user", "content": prompt}],
            "tools": tools, "tool_choice": "auto", "stream": True,
            "stream_options": {"include_usage": True}, "max_tokens": 8192,
            "thinking": {"type": "enabled"}, "reasoning_effort": model["thinkingLevel"]}
    if model["parameters"]:
        raise ValueError("This fixture expects default model parameters; inspect custom parameters explicitly before testing")
    headers = json.loads(provider["customHeaders"]) if provider["customHeaders"].strip() else {}
    headers.update({"Content-Type": "application/json", "Authorization": "Bearer " + provider["apiKey"]})
    request = urllib.request.Request(provider["endpoint"], data=json.dumps(body, ensure_ascii=False).encode(), headers=headers)
    chunks, tool_calls, usage, finish_reason = [], [], {}, None
    try:
        with urllib.request.urlopen(request, timeout=180) as response:
            for raw in response:
                line = raw.decode("utf-8").strip()
                if not line.startswith("data:"):
                    continue
                data = line[5:].strip()
                if data == "[DONE]":
                    break
                event = json.loads(data)
                if "error" in event:
                    raise RuntimeError("Provider returned an SSE error")
                if event.get("usage"):
                    usage = event["usage"]
                for choice in event.get("choices", []):
                    delta = choice.get("delta", {})
                    chunks.append(delta.get("content") or "")
                    tool_calls.extend(delta.get("tool_calls") or [])
                    if choice.get("finish_reason"):
                        finish_reason = choice["finish_reason"]
    except urllib.error.HTTPError as error:
        raise RuntimeError(f"Summary request failed with HTTP {error.code}; credentials and response body omitted") from None
    text = "".join(chunks).strip()
    stem = f"{case['id']}-{repeat}"
    (output / f"{stem}.md").write_text(text + "\n", encoding="utf-8")
    result = {"case": case["id"], "repeat": repeat, "model": model["id"],
              "prompt_sha256": hashlib.sha256(prompt.encode()).hexdigest(),
              "finish_reason": finish_reason, "usage": usage,
              "checks": inspect_summary(text, case, finish_reason, tool_calls),
              "semantic_review_required": True, "summary_file": stem + ".md"}
    (output / f"{stem}.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False), flush=True)
    return result


def main():
    """Run a small explicit live quality check without editing the application profile or executing tools."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preferences", type=Path, required=True)
    parser.add_argument("--prompt", type=Path, required=True, help="UTF-8 output of the export_summary_prompt Rust example")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeat", type=int, default=1)
    args = parser.parse_args()
    provider, model, binding = load_provider(args.preferences)
    prompt = args.prompt.read_text(encoding="utf-8-sig")
    fixtures = json.loads((Path(__file__).parent / "fixtures/chat_summary_quality.json").read_text(encoding="utf-8"))
    args.output.mkdir(parents=True, exist_ok=False)
    print(json.dumps({"model": model["id"], "configured_follows_chat": binding["followsChat"],
                      "synthetic_input_only": True, "full_application_e2e": False}, ensure_ascii=False), flush=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        futures = [pool.submit(run_case, provider, model, case, fixtures["tools"], prompt, args.output, repeat)
                   for case in fixtures["cases"] for repeat in range(1, args.repeat + 1)]
        results = [future.result() for future in concurrent.futures.as_completed(futures)]
    passed = all(all(result["checks"].values()) for result in results)
    print("Fact/structure checks:", "PASS" if passed else "FAIL; inspect generated summaries", flush=True)
    raise SystemExit(0 if passed else 1)


if __name__ == "__main__":
    main()

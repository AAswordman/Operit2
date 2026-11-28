"""Inspect one OpenAI-compatible model response without executing returned tool calls."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import time
import urllib.request


def first_turn(request_path, providers_path, provider_id, output_prefix):
    """Send a saved body once and retain its reasoning, output, calls, usage and timing."""
    output = Path(output_prefix)
    response_path = output.with_name(output.name + '.response.json')
    if response_path.exists():
        raise FileExistsError(f'Refusing to overwrite an existing response: {response_path}')
    payload = Path(request_path).read_bytes()
    body = json.loads(payload)
    providers = json.loads(Path(providers_path).read_text(encoding='utf-8'))
    provider = next(p for p in providers if p['id'] == provider_id)
    endpoint = provider['endpoint'].rstrip('/')
    if not endpoint.endswith('/chat/completions'):
        endpoint += '/chat/completions'
    if body.get('stream') is not True:
        raise ValueError('The saved request must enable stream')
    headers = {'Content-Type': 'application/json', 'Authorization': 'Bearer ' + provider['apiKey']}
    if provider.get('customHeaders'):
        headers.update(json.loads(provider['customHeaders']))
    request = urllib.request.Request(endpoint, data=payload, headers=headers)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    result = {'name': output.name, 'started_at': datetime.now(timezone.utc).isoformat(),
              'request_sha256': hashlib.sha256(payload).hexdigest(), 'model': body['model'],
              'reasoning_effort': body.get('reasoning_effort'), 'reasoning_content': '', 'content': '',
              'tool_calls': [], 'usage': None, 'finish_reason': None, 'executed_tool_calls': 0}
    calls = {}
    output.parent.mkdir(parents=True, exist_ok=True)
    start = time.monotonic()
    try:
        with opener.open(request, timeout=180) as response:
            for line in response:
                if not line.startswith(b'data: '):
                    continue
                data = line[6:].strip()
                if data == b'[DONE]':
                    break
                event = json.loads(data)
                if event.get('error'):
                    raise RuntimeError('Provider returned a streaming error')
                if 'ttfb_seconds' not in result:
                    result['ttfb_seconds'] = time.monotonic() - start
                if event.get('usage'):
                    result['usage'] = event['usage']
                result['response_model'] = event.get('model', result.get('response_model'))
                for choice in event.get('choices', []):
                    delta = choice.get('delta', {})
                    result['reasoning_content'] += delta.get('reasoning_content') or ''
                    result['content'] += delta.get('content') or ''
                    if choice.get('finish_reason'):
                        result['finish_reason'] = choice['finish_reason']
                    for call in delta.get('tool_calls', []):
                        target = calls.setdefault(call['index'], {'id': '', 'type': 'function', 'function': {'name': '', 'arguments': ''}})
                        target['id'] += call.get('id', '')
                        for key in ('name', 'arguments'):
                            target['function'][key] += call.get('function', {}).get(key, '')
    except Exception as error:
        result['error_type'] = type(error).__name__
        raise
    finally:
        result['seconds'] = time.monotonic() - start
        result['tool_calls'] = [calls[key] for key in sorted(calls)]
        result['reasoning_chars'] = len(result['reasoning_content'])
        result['content_chars'] = len(result['content'])
        response_path.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
        output.with_name(output.name + '.thinking.txt').write_text(result['reasoning_content'], encoding='utf-8')
        output.with_name(output.name + '.output.txt').write_text(
            result['content'] + '\n\n' + json.dumps(result['tool_calls'], ensure_ascii=False, indent=2), encoding='utf-8')
    return result


def main():
    """Require explicit request, credential source and output paths for a single API call."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--request', required=True, type=Path)
    parser.add_argument('--providers', required=True, type=Path)
    parser.add_argument('--provider', default='DEEPSEEK')
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    result = first_turn(args.request, args.providers, args.provider, args.output)
    print(json.dumps({key: value for key, value in result.items()
                      if key not in ('reasoning_content', 'content', 'tool_calls')}, ensure_ascii=False))
    print('Proposed tools:', ', '.join(call['function']['name'] for call in result['tool_calls']))


if __name__ == '__main__':
    main()

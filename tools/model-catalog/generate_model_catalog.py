#!/usr/bin/env python3
import argparse
import json
import ssl
import sys
import urllib.request
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

import certifi


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
TARGET = REPO_ROOT / "core" / "crates" / "operit-runtime" / "src" / "data" / "collects" / "ModelCatalog.rs"

USER_AGENT = "operit-model-catalog-generator/1.0"
MODELS_DEV_URL = "https://models.dev/api.json"


@dataclass(frozen=True)
class SourceProvider:
    provider_type_id: str
    source_id: str
    source: str


PROVIDERS = [
    SourceProvider(
        "DEEPSEEK",
        "deepseek",
        "models.dev",
    ),
    SourceProvider(
        "OPENAI",
        "openai",
        "models.dev",
    ),
    SourceProvider(
        "GOOGLE",
        "google",
        "models.dev",
    ),
    SourceProvider(
        "ANTHROPIC",
        "anthropic",
        "models.dev",
    ),
    SourceProvider(
        "MISTRAL",
        "mistral",
        "models.dev",
    ),
    SourceProvider(
        "OPENROUTER",
        "openrouter",
        "models.dev",
    ),
    SourceProvider(
        "SILICONFLOW",
        "siliconflow-cn",
        "models.dev",
    ),
]


def fetch_json(url: str) -> Any:
    ctx = ssl.create_default_context(cafile=certifi.where())
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=60, context=ctx) as response:
        return json.load(response)


def required_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{path} must be an object")
    return value


def required_list(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{path} must be a list")
    return value


def required_string(value: Any, path: str) -> str:
    if not isinstance(value, str) or value == "":
        raise ValueError(f"{path} must be a non-empty string")
    return value


def required_bool(value: Any, path: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{path} must be a bool")
    return value


def required_number(value: Any, path: str) -> Decimal:
    if isinstance(value, bool):
        raise ValueError(f"{path} must be a number")
    if isinstance(value, (int, float, str)):
        try:
            return Decimal(str(value))
        except InvalidOperation as error:
            raise ValueError(f"{path} must be a number") from error
    raise ValueError(f"{path} must be a number")


def optional_number(value: Any, path: str) -> Decimal | None:
    if value is None:
        return None
    return required_number(value, path)


def format_decimal(value: Decimal) -> str:
    text = format(value.normalize(), "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    if text == "-0":
        text = "0"
    return text


def tokens_to_k(value: Decimal) -> Decimal:
    return value / Decimal("1000")


def bool_field(value: bool) -> str:
    return "true" if value else "false"


def clean_field(value: str) -> str:
    if "\n" in value or "\r" in value or "|" in value:
        raise ValueError(f"catalog field contains unsupported separator: {value!r}")
    return value


def row(fields: list[str]) -> str:
    return "|".join(clean_field(field) for field in fields)


def has_text_output(output_modalities: list[Any], path: str) -> bool:
    modalities = [required_string(item, f"{path}[{index}]") for index, item in enumerate(output_modalities)]
    return "text" in modalities


def modality_flags(input_modalities: list[Any], path: str) -> tuple[bool, bool, bool]:
    modalities = [required_string(item, f"{path}[{index}]") for index, item in enumerate(input_modalities)]
    return (
        "image" in modalities,
        "audio" in modalities,
        "video" in modalities,
    )


def model_row(
    provider_type_id: str,
    api_name: str,
    input_price: Decimal,
    cached_input_price: Decimal | None,
    output_price: Decimal,
    currency: str,
    context_tokens: Decimal,
    direct_image: bool,
    direct_audio: bool,
    direct_video: bool,
    tool_call: bool,
) -> str:
    cached = "" if cached_input_price is None else format_decimal(cached_input_price)
    return row(
        [
            provider_type_id,
            api_name,
            "TOKEN",
            format_decimal(input_price),
            cached,
            format_decimal(output_price),
            "0",
            currency,
            format_decimal(tokens_to_k(context_tokens)),
            "false",
            bool_field(direct_image),
            bool_field(direct_audio),
            bool_field(direct_video),
            "false",
            bool_field(tool_call),
            bool_field(tool_call),
        ]
    )


def collect_models_dev(provider: SourceProvider, data: dict[str, Any]) -> list[str]:
    provider_data = required_object(data.get(provider.source_id), f"models.dev.{provider.source_id}")
    models = required_object(provider_data.get("models"), f"models.dev.{provider.source_id}.models")
    rows: list[str] = []
    for model_id in sorted(models.keys(), key=str.lower):
        model = required_object(models[model_id], f"{provider.source_id}.{model_id}")
        modalities = required_object(model.get("modalities"), f"{provider.source_id}.{model_id}.modalities")
        input_modalities = required_list(modalities.get("input"), f"{provider.source_id}.{model_id}.modalities.input")
        output_modalities = required_list(modalities.get("output"), f"{provider.source_id}.{model_id}.modalities.output")
        if not has_text_output(output_modalities, f"{provider.source_id}.{model_id}.modalities.output"):
            continue

        cost_value = model.get("cost")
        if not isinstance(cost_value, dict) or "input" not in cost_value or "output" not in cost_value:
            continue

        limit = required_object(model.get("limit"), f"{provider.source_id}.{model_id}.limit")
        cost = required_object(cost_value, f"{provider.source_id}.{model_id}.cost")
        image, audio, video = modality_flags(input_modalities, f"{provider.source_id}.{model_id}.modalities.input")
        rows.append(
            model_row(
                provider.provider_type_id,
                required_string(model.get("id"), f"{provider.source_id}.{model_id}.id"),
                required_number(cost.get("input"), f"{provider.source_id}.{model_id}.cost.input"),
                optional_number(cost.get("cache_read"), f"{provider.source_id}.{model_id}.cost.cache_read"),
                required_number(cost.get("output"), f"{provider.source_id}.{model_id}.cost.output"),
                "USD",
                required_number(limit.get("context"), f"{provider.source_id}.{model_id}.limit.context"),
                image,
                audio,
                video,
                required_bool(model.get("tool_call"), f"{provider.source_id}.{model_id}.tool_call"),
            )
        )
    return rows


def generate_rows() -> list[str]:
    models_dev = required_object(fetch_json(MODELS_DEV_URL), "models.dev")
    model_rows: list[str] = []
    for provider in PROVIDERS:
        if provider.source == "models.dev":
            rows = collect_models_dev(provider, models_dev)
        else:
            raise ValueError(f"unknown source: {provider.source}")
        if len(rows) == 0:
            raise ValueError(f"{provider.provider_type_id} produced zero rows")
        model_rows.extend(rows)
    return model_rows


def render(model_data: list[str]) -> str:
    model_block = "\n".join(model_data)
    return f"""pub const MODEL_CATALOG_MODEL_ROWS: &str = r#"
{model_block}
"#;
"""


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate compact model catalog row data.")
    parser.add_argument("--check", action="store_true", help="Verify that ModelCatalog.rs matches generated output.")
    args = parser.parse_args()

    generated = render(generate_rows())
    if args.check:
        current = TARGET.read_text(encoding="utf-8")
        if current != generated:
            print(f"{TARGET} is not up to date", file=sys.stderr)
            return 1
        print(f"{TARGET} is up to date")
        return 0

    TARGET.write_text(generated, encoding="utf-8", newline="\n")
    print(f"wrote {TARGET}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

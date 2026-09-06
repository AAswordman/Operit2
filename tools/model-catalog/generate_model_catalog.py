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
TARGET = REPO_ROOT / "core" / "crates" / "foundation" / "model" / "collects" / "ModelCatalog.rs"

USER_AGENT = "operit-model-catalog-generator/1.0"
MODELS_DEV_URL = "https://models.dev/api.json"
LITELLM_PRICING_URL = (
    "https://raw.githubusercontent.com/BerriAI/litellm/main/"
    "model_prices_and_context_window.json"
)
CCSWITCH_CODEX_PROVIDER_PRESETS_URL = (
    "https://raw.githubusercontent.com/farion1231/cc-switch/main/src/config/"
    "codexProviderPresets.ts"
)
CCSWITCH_SCHEMA_URL = (
    "https://raw.githubusercontent.com/farion1231/cc-switch/main/src-tauri/src/database/schema.rs"
)


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
    SourceProvider(
        "MOONSHOT",
        "Kimi",
        "ccswitch",
    ),
    SourceProvider(
        "DEEPSEEK",
        "DeepSeek",
        "ccswitch",
    ),
    SourceProvider(
        "ZHIPU",
        "Zhipu GLM",
        "ccswitch",
    ),
    SourceProvider(
        "BAIDU",
        "Baidu Qianfan Token Plan",
        "ccswitch",
    ),
    SourceProvider(
        "ALIYUN",
        "Bailian",
        "ccswitch",
    ),
    SourceProvider(
        "ALIYUN",
        "QwenCloud",
        "ccswitch",
    ),
    SourceProvider(
        "ALIYUN",
        "QwenCloud For Coding",
        "ccswitch",
    ),
    SourceProvider(
        "ALIYUN",
        "QwenCloud Token Plan",
        "ccswitch",
    ),
    SourceProvider(
        "DOUBAO",
        "DouBaoSeed",
        "ccswitch",
    ),
    SourceProvider(
        "SILICONFLOW",
        "SiliconFlow",
        "ccswitch",
    ),
    SourceProvider(
        "SILICONFLOW",
        "SiliconFlow en",
        "ccswitch",
    ),
    SourceProvider(
        "MIMO",
        "Xiaomi MiMo",
        "ccswitch",
    ),
    SourceProvider(
        "MIMO",
        "Xiaomi MiMo Token Plan (China)",
        "ccswitch",
    ),
    SourceProvider(
        "NOVITA",
        "Novita AI",
        "ccswitch",
    ),
    SourceProvider(
        "NVIDIA",
        "Nvidia",
        "ccswitch",
    ),
]

LITELLM_HISTORY_PROVIDERS = [
    SourceProvider(
        "DEEPSEEK",
        "deepseek",
        "litellm",
    ),
    SourceProvider(
        "OPENAI",
        "openai",
        "litellm",
    ),
    SourceProvider(
        "GOOGLE",
        "gemini",
        "litellm",
    ),
    SourceProvider(
        "ANTHROPIC",
        "anthropic",
        "litellm",
    ),
    SourceProvider(
        "MISTRAL",
        "mistral",
        "litellm",
    ),
    SourceProvider(
        "OPENROUTER",
        "openrouter",
        "litellm",
    ),
]


# Fetch JSON content from a URL using the project certificate bundle.
def fetch_json(url: str) -> Any:
    ctx = ssl.create_default_context(cafile=certifi.where())
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=60, context=ctx) as response:
        return json.load(response)


# Fetch UTF-8 text content from a URL using the project certificate bundle.
def fetch_text(url: str) -> str:
    ctx = ssl.create_default_context(cafile=certifi.where())
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=60, context=ctx) as response:
        return response.read().decode("utf-8")


@dataclass(frozen=True)
class Token:
    kind: str
    value: str


@dataclass(frozen=True)
class CcSwitchModel:
    model: str
    context_window: Decimal | None
    input_modalities: tuple[str, ...] | None
    supports_parallel_tool_calls: bool


@dataclass(frozen=True)
class CcSwitchPreset:
    name: str
    models: tuple[CcSwitchModel, ...]


@dataclass(frozen=True)
class CcSwitchPricing:
    display_name: str
    input_price: Decimal
    cached_input_price: Decimal
    output_price: Decimal


# Require a JSON value to be an object.
def required_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{path} must be an object")
    return value


# Require a JSON value to be a list.
def required_list(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{path} must be a list")
    return value


# Require a JSON value to be a non-empty string.
def required_string(value: Any, path: str) -> str:
    if not isinstance(value, str) or value == "":
        raise ValueError(f"{path} must be a non-empty string")
    return value


# Require a JSON value to be a boolean.
def required_bool(value: Any, path: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{path} must be a bool")
    return value


# Require a JSON value to be a decimal-compatible number.
def required_number(value: Any, path: str) -> Decimal:
    if isinstance(value, bool):
        raise ValueError(f"{path} must be a number")
    if isinstance(value, (int, float, str)):
        try:
            return Decimal(str(value))
        except InvalidOperation as error:
            raise ValueError(f"{path} must be a number") from error
    raise ValueError(f"{path} must be a number")


# Parse an optional JSON number as a Decimal.
def optional_number(value: Any, path: str) -> Decimal | None:
    if value is None:
        return None
    return required_number(value, path)


# Require a parsed token at the current stream position.
def required_token(tokens: list[Token], index: int, value: str, path: str) -> int:
    if index >= len(tokens) or tokens[index].value != value:
        actual = "<eof>" if index >= len(tokens) else tokens[index].value
        raise ValueError(f"{path} expected {value!r}, got {actual!r}")
    return index + 1


# Parse a decimal-compatible token value.
def token_number(token: Token, path: str) -> Decimal:
    if token.kind != "number":
        raise ValueError(f"{path} must be a number")
    try:
        return Decimal(token.value)
    except InvalidOperation as error:
        raise ValueError(f"{path} must be a number") from error


# Format a Decimal for compact Rust row output.
def format_decimal(value: Decimal) -> str:
    text = format(value.normalize(), "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    if text == "-0":
        text = "0"
    return text


# Convert token counts into thousands of tokens.
def tokens_to_k(value: Decimal) -> Decimal:
    return value / Decimal("1000")


# Render a Rust boolean literal.
def bool_field(value: bool) -> str:
    return "true" if value else "false"


# Validate a catalog field before row joining.
def clean_field(value: str) -> str:
    if "\n" in value or "\r" in value or "|" in value:
        raise ValueError(f"catalog field contains unsupported separator: {value!r}")
    return value


# Join catalog fields with the row separator.
def row(fields: list[str]) -> str:
    return "|".join(clean_field(field) for field in fields)


# Determine whether output modalities include text.
def has_text_output(output_modalities: list[Any], path: str) -> bool:
    modalities = [required_string(item, f"{path}[{index}]") for index, item in enumerate(output_modalities)]
    return "text" in modalities


# Convert input modalities into direct media capability flags.
def modality_flags(input_modalities: list[Any], path: str) -> tuple[bool, bool, bool]:
    modalities = [required_string(item, f"{path}[{index}]") for index, item in enumerate(input_modalities)]
    return (
        "image" in modalities,
        "audio" in modalities,
        "video" in modalities,
    )


CCSWITCH_CONFIRMED_TEXT_ONLY_TAILS = {
    "ark-code-latest",
    "deepseek-chat",
    "deepseek-reasoner",
    "deepseek-v4-flash",
    "deepseek-v4-pro",
    "glm-5.1",
    "glm-5.2",
    "glm-5.3",
    "kat-coder",
    "kat-coder-pro",
    "kat-coder-pro v1",
    "kat-coder-pro v2",
    "kat-coder-pro-v1",
    "kat-coder-pro-v2",
    "ling-2.5-1t",
    "longcat-2.0",
    "longcat-flash-chat",
    "minimax-m2.7",
    "minimax-m2.7-highspeed",
    "mimo-v2.5-pro",
    "qwen3-coder-480b",
    "qwen3-coder-480b-a35b-instruct",
    "qwen3-coder-flash",
    "qwen3-coder-next",
    "qwen3-coder-plus",
    "step-3.5-flash",
    "step-3.5-flash-2603",
    "us.deepseek.r1-v1",
}


# Normalize a model id using cc-switch capability lookup semantics.
def normalize_model_id_for_ccswitch_capabilities(model_id: str) -> str:
    normalized = model_id.strip()
    if normalized.startswith("models/"):
        normalized = normalized[len("models/") :]
    normalized = normalized.strip().lower()
    context_marker = "[1m]"
    if normalized.endswith(context_marker):
        normalized = normalized[: -len(context_marker)].strip()
    return normalized


# Check whether cc-switch classifies a model as text-only.
def is_ccswitch_confirmed_text_only_model(model_id: str) -> bool:
    normalized = normalize_model_id_for_ccswitch_capabilities(model_id)
    tail = normalized.rsplit("/", 1)[-1]
    return tail in CCSWITCH_CONFIRMED_TEXT_ONLY_TAILS


# Convert parsed modality names into direct media capability flags.
def parsed_modality_flags(
    model_id: str,
    input_modalities: tuple[str, ...] | None,
) -> tuple[bool, bool, bool]:
    if input_modalities is None:
        modalities = (
            ("text",)
            if is_ccswitch_confirmed_text_only_model(model_id)
            else ("text", "image")
        )
    else:
        modalities = input_modalities
    return (
        "image" in modalities,
        "audio" in modalities,
        "video" in modalities,
    )


# Build one compact model catalog row.
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


# Tokenize the TypeScript subset used by cc-switch provider presets.
def tokenize_ts(source: str) -> list[Token]:
    tokens: list[Token] = []
    index = 0
    while index < len(source):
        char = source[index]
        if char.isspace():
            index += 1
            continue
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            index = len(source) if end == -1 else end + 1
            continue
        if source.startswith("/*", index):
            end = source.find("*/", index + 2)
            if end == -1:
                raise ValueError("unterminated block comment")
            index = end + 2
            continue
        if char in ("'", '"', "`"):
            value, index = read_ts_string(source, index)
            tokens.append(Token("string", value))
            continue
        if char.isdigit() or (char == "-" and index + 1 < len(source) and source[index + 1].isdigit()):
            start = index
            index += 1
            while index < len(source) and (source[index].isdigit() or source[index] == "."):
                index += 1
            tokens.append(Token("number", source[start:index]))
            continue
        if char.isalpha() or char == "_" or ord(char) > 127:
            start = index
            index += 1
            while index < len(source):
                current = source[index]
                if current.isalnum() or current in ("_", "-") or ord(current) > 127:
                    index += 1
                else:
                    break
            tokens.append(Token("identifier", source[start:index]))
            continue
        if char in "{}[]():,=;?":
            tokens.append(Token("punct", char))
            index += 1
            continue
        index += 1
    return tokens


# Read one quoted TypeScript string token.
def read_ts_string(source: str, index: int) -> tuple[str, int]:
    quote = source[index]
    index += 1
    chars: list[str] = []
    while index < len(source):
        char = source[index]
        if char == "\\":
            if index + 1 >= len(source):
                raise ValueError("unterminated string escape")
            chars.append(source[index + 1])
            index += 2
            continue
        if char == quote:
            return "".join(chars), index + 1
        chars.append(char)
        index += 1
    raise ValueError("unterminated string")


# Skip one TypeScript expression value.
def skip_ts_value(tokens: list[Token], index: int) -> int:
    closers: list[str] = []
    while index < len(tokens):
        value = tokens[index].value
        if not closers and value in (",", "}", "]", ")"):
            return index
        if value == "{":
            closers.append("}")
        elif value == "[":
            closers.append("]")
        elif value == "(":
            closers.append(")")
        elif value in ("}", "]", ")"):
            if not closers or closers[-1] != value:
                return index
            closers.pop()
        index += 1
    return index


# Find the cc-switch provider preset array token index.
def ccswitch_presets_array_index(tokens: list[Token]) -> int:
    for index, token in enumerate(tokens):
        if token.kind == "identifier" and token.value == "codexProviderPresets":
            probe = index + 1
            while probe < len(tokens) and tokens[probe].value != "=":
                probe += 1
            probe = required_token(tokens, probe, "=", "ccswitch.codexProviderPresets")
            return required_token(tokens, probe, "[", "ccswitch.codexProviderPresets") - 1
    raise ValueError("ccswitch.codexProviderPresets not found")


# Parse all cc-switch Codex provider presets.
def parse_ccswitch_presets(source: str) -> dict[str, CcSwitchPreset]:
    tokens = tokenize_ts(source)
    index = required_token(tokens, ccswitch_presets_array_index(tokens), "[", "ccswitch.codexProviderPresets")
    presets: dict[str, CcSwitchPreset] = {}
    while index < len(tokens):
        value = tokens[index].value
        if value == "]":
            return presets
        if value == ",":
            index += 1
            continue
        if value != "{":
            raise ValueError(f"ccswitch.codexProviderPresets expected object, got {value!r}")
        preset, index = parse_ccswitch_preset(tokens, index)
        if preset.models:
            presets[preset.name] = preset
    raise ValueError("ccswitch.codexProviderPresets array is not closed")


# Parse one cc-switch provider preset object.
def parse_ccswitch_preset(tokens: list[Token], index: int) -> tuple[CcSwitchPreset, int]:
    index = required_token(tokens, index, "{", "ccswitch.preset")
    name = ""
    models: tuple[CcSwitchModel, ...] = ()
    while index < len(tokens):
        value = tokens[index].value
        if value == "}":
            return CcSwitchPreset(name, models), index + 1
        if value == ",":
            index += 1
            continue
        key = tokens[index].value
        index += 1
        index = required_token(tokens, index, ":", f"ccswitch.preset.{key}")
        if key == "name":
            if tokens[index].kind != "string":
                raise ValueError("ccswitch.preset.name must be a string")
            name = tokens[index].value
            index += 1
        elif key == "modelCatalog":
            models, index = parse_ccswitch_model_catalog_call(tokens, index)
        else:
            index = skip_ts_value(tokens, index)
    raise ValueError("ccswitch.preset object is not closed")


# Parse the modelCatalog([...]) call used by cc-switch presets.
def parse_ccswitch_model_catalog_call(
    tokens: list[Token],
    index: int,
) -> tuple[tuple[CcSwitchModel, ...], int]:
    if tokens[index].kind != "identifier" or tokens[index].value != "modelCatalog":
        raise ValueError("ccswitch.modelCatalog must call modelCatalog")
    index = required_token(tokens, index + 1, "(", "ccswitch.modelCatalog")
    models, index = parse_ccswitch_model_array(tokens, index)
    index = required_token(tokens, index, ")", "ccswitch.modelCatalog")
    return tuple(models), index


# Parse the array passed to a cc-switch modelCatalog call.
def parse_ccswitch_model_array(tokens: list[Token], index: int) -> tuple[list[CcSwitchModel], int]:
    index = required_token(tokens, index, "[", "ccswitch.modelCatalog.models")
    models: list[CcSwitchModel] = []
    while index < len(tokens):
        value = tokens[index].value
        if value == "]":
            return models, index + 1
        if value == ",":
            index += 1
            continue
        if tokens[index].kind == "string":
            models.append(
                CcSwitchModel(
                    tokens[index].value,
                    None,
                    None,
                    False,
                )
            )
            index += 1
        elif value == "{":
            model, index = parse_ccswitch_model_object(tokens, index)
            models.append(model)
        else:
            raise ValueError(f"ccswitch.modelCatalog.models expected model entry, got {value!r}")
    raise ValueError("ccswitch.modelCatalog.models array is not closed")


# Parse one cc-switch catalog model object.
def parse_ccswitch_model_object(tokens: list[Token], index: int) -> tuple[CcSwitchModel, int]:
    index = required_token(tokens, index, "{", "ccswitch.model")
    model = ""
    context_window: Decimal | None = None
    input_modalities: tuple[str, ...] | None = None
    supports_parallel_tool_calls = False
    while index < len(tokens):
        value = tokens[index].value
        if value == "}":
            if model == "":
                raise ValueError("ccswitch.model.model must be a string")
            return (
                CcSwitchModel(
                    model,
                    context_window,
                    input_modalities,
                    supports_parallel_tool_calls,
                ),
                index + 1,
            )
        if value == ",":
            index += 1
            continue
        key = tokens[index].value
        index += 1
        index = required_token(tokens, index, ":", f"ccswitch.model.{key}")
        if key == "model":
            if tokens[index].kind != "string":
                raise ValueError("ccswitch.model.model must be a string")
            model = tokens[index].value
            index += 1
        elif key == "contextWindow":
            context_window = token_number(tokens[index], "ccswitch.model.contextWindow")
            index += 1
        elif key == "inputModalities":
            input_modalities, index = parse_ccswitch_string_array(tokens, index)
        elif key == "supportsParallelToolCalls":
            if tokens[index].value not in ("true", "false"):
                raise ValueError("ccswitch.model.supportsParallelToolCalls must be a bool")
            supports_parallel_tool_calls = tokens[index].value == "true"
            index += 1
        else:
            index = skip_ts_value(tokens, index)
    raise ValueError("ccswitch.model object is not closed")


# Parse a TypeScript string array.
def parse_ccswitch_string_array(tokens: list[Token], index: int) -> tuple[tuple[str, ...], int]:
    index = required_token(tokens, index, "[", "ccswitch.stringArray")
    values: list[str] = []
    while index < len(tokens):
        value = tokens[index].value
        if value == "]":
            return tuple(values), index + 1
        if value == ",":
            index += 1
            continue
        if tokens[index].kind != "string":
            raise ValueError("ccswitch.stringArray item must be a string")
        values.append(tokens[index].value)
        index += 1
    raise ValueError("ccswitch.stringArray is not closed")


# Find the cc-switch seed pricing array token index.
def ccswitch_pricing_array_index(tokens: list[Token]) -> int:
    for index, token in enumerate(tokens):
        if token.kind == "identifier" and token.value == "pricing_data":
            probe = index + 1
            while probe < len(tokens) and tokens[probe].value != "=":
                probe += 1
            probe = required_token(tokens, probe, "=", "ccswitch.pricing_data")
            return required_token(tokens, probe, "[", "ccswitch.pricing_data") - 1
    raise ValueError("ccswitch.pricing_data not found")


# Parse one Rust tuple of string literals.
def parse_ccswitch_string_tuple(
    tokens: list[Token],
    index: int,
    expected_count: int,
    path: str,
) -> tuple[tuple[str, ...], int]:
    index = required_token(tokens, index, "(", path)
    values: list[str] = []
    while index < len(tokens):
        value = tokens[index].value
        if value == ")":
            if len(values) != expected_count:
                raise ValueError(f"{path} expected {expected_count} string values, got {len(values)}")
            return tuple(values), index + 1
        if value == ",":
            index += 1
            continue
        if tokens[index].kind != "string":
            raise ValueError(f"{path} item must be a string")
        values.append(tokens[index].value)
        index += 1
    raise ValueError(f"{path} tuple is not closed")


# Parse cc-switch seed pricing rows from schema.rs.
def parse_ccswitch_pricing(source: str) -> dict[str, CcSwitchPricing]:
    tokens = tokenize_ts(source)
    index = required_token(tokens, ccswitch_pricing_array_index(tokens), "[", "ccswitch.pricing_data")
    prices: dict[str, CcSwitchPricing] = {}
    while index < len(tokens):
        value = tokens[index].value
        if value == "]":
            return prices
        if value == ",":
            index += 1
            continue
        values, index = parse_ccswitch_string_tuple(tokens, index, 6, "ccswitch.pricing")
        model_id, display_name, input_price, output_price, cached_input_price, _cache_write_price = values
        prices[model_id.strip().lower()] = CcSwitchPricing(
            display_name,
            required_number(input_price, f"ccswitch.pricing.{model_id}.input"),
            required_number(cached_input_price, f"ccswitch.pricing.{model_id}.cache_read"),
            required_number(output_price, f"ccswitch.pricing.{model_id}.output"),
        )
    raise ValueError("ccswitch.pricing_data array is not closed")


# Check whether a model id is a non-priced placeholder token.
def is_placeholder_pricing_model(model_id: str) -> bool:
    normalized = model_id.strip().lower()
    return normalized == "" or normalized in ("unknown", "null", "none")


# Clean a model id before applying cc-switch pricing key transforms.
def clean_model_id_for_ccswitch_pricing(model_id: str) -> str:
    terminal = model_id.rsplit("/", 1)[-1]
    normalized = terminal.split(":", 1)[0].strip().replace("@", "-").lower()
    context_marker = "[1m]"
    if normalized.endswith(context_marker):
        normalized = normalized[: -len(context_marker)].strip()
    return normalized


# Append a candidate key while preserving discovery order.
def push_unique_candidate(candidates: list[str], candidate: str) -> bool:
    if candidate == "" or candidate in candidates:
        return False
    candidates.append(candidate)
    return True


# Remove cc-switch known provider namespaces from a model id.
def strip_known_model_namespace(model_id: str) -> str | None:
    claude_pos = model_id.rfind("claude-")
    if claude_pos > 0:
        return model_id[claude_pos:]

    for marker in (
        "openai.",
        "anthropic.",
        "google.",
        "moonshot.",
        "moonshotai.",
        "bedrock.",
        "global.",
    ):
        if model_id.startswith(marker):
            return model_id[len(marker) :]
    return None


# Remove Claude Desktop wrapping around non-Anthropic model ids.
def strip_claude_desktop_non_anthropic_prefix(model_id: str) -> str | None:
    markers = (
        "abab",
        "ark-code",
        "arctic",
        "astron",
        "codex",
        "command-r",
        "deepseek",
        "doubao",
        "ernie",
        "gemini",
        "gemma",
        "glm",
        "gpt",
        "grok",
        "hermes",
        "hy3",
        "hunyuan",
        "jamba",
        "kimi",
        "lfm",
        "llama",
        "longcat",
        "mercury",
        "mimo",
        "minimax",
        "mistral",
        "mixtral",
        "moonshot",
        "nemotron",
        "nova-",
        "openai",
        "qianfan",
        "qwen",
        "seed-",
        "solar",
        "stepfun",
    )
    if not model_id.startswith("claude-"):
        return None
    rest = model_id[len("claude-") :]
    for marker in markers:
        if rest.startswith(marker):
            return rest
    return None


# Remove Bedrock-style numeric model version suffixes.
def strip_bedrock_model_version_suffix(model_id: str) -> str | None:
    if "-v" not in model_id:
        return None
    base, suffix = model_id.rsplit("-v", 1)
    if base != "" and suffix != "" and suffix.isdigit():
        return base
    return None


# Remove date suffixes supported by cc-switch pricing lookup.
def strip_model_date_suffix(model_id: str) -> str | None:
    if len(model_id) > 11:
        start = len(model_id) - 11
        suffix = model_id[start:]
        is_iso_date = (
            suffix[0] == "-"
            and suffix[1:5].isdigit()
            and suffix[5] == "-"
            and suffix[6:8].isdigit()
            and suffix[8] == "-"
            and suffix[9:11].isdigit()
        )
        if is_iso_date:
            return model_id[:start]

    if "-" not in model_id:
        return None
    base, suffix = model_id.rsplit("-", 1)
    if base == "" or not suffix.isdigit():
        return None
    if len(suffix) == 8:
        return base
    if len(suffix) == 6:
        month = int(suffix[2:4])
        day = int(suffix[4:6])
        if 1 <= month <= 12 and 1 <= day <= 31:
            return base
    return None


# Remove reasoning effort suffixes supported by cc-switch pricing lookup.
def strip_reasoning_effort_suffix(model_id: str) -> str | None:
    for suffix in ("-minimal", "-low", "-medium", "-high", "-xhigh"):
        if model_id.endswith(suffix):
            stripped = model_id[: -len(suffix)]
            if stripped != "":
                return stripped
    return None


# Build pricing candidate keys using cc-switch lookup semantics.
def ccswitch_pricing_candidates(model_id: str) -> list[str]:
    cleaned = clean_model_id_for_ccswitch_pricing(model_id)
    if is_placeholder_pricing_model(cleaned):
        return []

    candidates: list[str] = []
    queue = [cleaned]
    while queue:
        candidate = queue.pop()
        if not push_unique_candidate(candidates, candidate):
            continue

        for transform in (
            strip_known_model_namespace,
            strip_claude_desktop_non_anthropic_prefix,
            strip_bedrock_model_version_suffix,
            strip_model_date_suffix,
            strip_reasoning_effort_suffix,
        ):
            stripped = transform(candidate)
            if stripped is not None:
                queue.append(stripped)
        if candidate.startswith("claude-") and "." in candidate:
            queue.append(candidate.replace(".", "-"))

    return candidates


# Check whether cc-switch would query longer priced variants for a candidate.
def should_try_ccswitch_pricing_prefix_match(model_id: str) -> bool:
    dash_count = model_id.count("-")

    if model_id.startswith("claude-"):
        return dash_count >= 3

    if model_id.startswith(("o1", "o3", "o4", "o5")):
        return dash_count >= 1

    return model_id.startswith(
        (
            "gpt-",
            "gemini-",
            "deepseek-",
            "qwen-",
            "glm-",
            "kimi-",
            "minimax-",
        )
    ) and dash_count >= 2


# Resolve a cc-switch model id to a seed pricing row.
def ccswitch_pricing_for_model(
    prices: dict[str, CcSwitchPricing],
    model_id: str,
) -> CcSwitchPricing | None:
    candidates = ccswitch_pricing_candidates(model_id)
    for candidate in candidates:
        price = prices.get(candidate)
        if price is not None:
            return price

    for candidate in candidates:
        if not should_try_ccswitch_pricing_prefix_match(candidate):
            continue
        prefix = f"{candidate}-"
        matches = [key for key in prices if key.startswith(prefix)]
        if matches:
            return prices[min(matches, key=len)]

    return None


# Convert a LiteLLM per-token price into catalog per-million units.
def litellm_price_per_million(value: Any, path: str) -> Decimal:
    return required_number(value, path) * Decimal("1000000")


# Read a LiteLLM optional per-token price in catalog per-million units.
def optional_litellm_price_per_million(value: Any, path: str) -> Decimal | None:
    if value is None:
        return None
    return litellm_price_per_million(value, path)


# Read the LiteLLM cache-read price field variants.
def litellm_cached_input_price(entry: dict[str, Any], path: str) -> Decimal | None:
    for field in ("cache_read_input_token_cost", "input_cost_per_token_cache_hit"):
        if field in entry:
            return optional_litellm_price_per_million(entry.get(field), f"{path}.{field}")
    return None


# Convert a LiteLLM model key into the provider-local model id.
def litellm_api_name(provider: SourceProvider, model_key: str) -> str:
    prefix = f"{provider.source_id}/"
    if model_key.lower().startswith(prefix):
        return model_key[len(prefix) :]
    return model_key


# Check whether a LiteLLM row carries a chat token price.
def is_litellm_priced_chat_model(entry: dict[str, Any]) -> bool:
    return (
        entry.get("mode") == "chat"
        and "input_cost_per_token" in entry
        and "output_cost_per_token" in entry
        and "max_input_tokens" in entry
    )


# Convert LiteLLM modality fields into direct media capability flags.
def litellm_modality_flags(entry: dict[str, Any], path: str) -> tuple[bool, bool, bool]:
    modalities_value = entry.get("supported_modalities")
    modalities: set[str] = set()
    if modalities_value is not None:
        for index, item in enumerate(required_list(modalities_value, f"{path}.supported_modalities")):
            modalities.add(required_string(item, f"{path}.supported_modalities[{index}]").lower())
    direct_image = required_bool(entry.get("supports_vision"), f"{path}.supports_vision") if "supports_vision" in entry else "image" in modalities
    direct_audio = required_bool(entry.get("supports_audio_input"), f"{path}.supports_audio_input") if "supports_audio_input" in entry else "audio" in modalities
    direct_video = required_bool(entry.get("supports_video_input"), f"{path}.supports_video_input") if "supports_video_input" in entry else "video" in modalities
    return direct_image, direct_audio, direct_video


# Collect model rows for one LiteLLM historical provider.
def collect_litellm_history(provider: SourceProvider, data: dict[str, Any]) -> list[str]:
    rows: list[str] = []
    for model_key in sorted(data.keys(), key=str.lower):
        entry = required_object(data[model_key], f"litellm.{model_key}")
        if entry.get("litellm_provider") != provider.source_id:
            continue
        if not is_litellm_priced_chat_model(entry):
            continue

        path = f"litellm.{model_key}"
        image, audio, video = litellm_modality_flags(entry, path)
        rows.append(
            model_row(
                provider.provider_type_id,
                litellm_api_name(provider, model_key),
                litellm_price_per_million(entry.get("input_cost_per_token"), f"{path}.input_cost_per_token"),
                litellm_cached_input_price(entry, path),
                litellm_price_per_million(entry.get("output_cost_per_token"), f"{path}.output_cost_per_token"),
                "USD",
                required_number(entry.get("max_input_tokens"), f"{path}.max_input_tokens"),
                image,
                audio,
                video,
                required_bool(entry.get("supports_function_calling"), f"{path}.supports_function_calling") if "supports_function_calling" in entry else False,
            )
        )
    return rows


# Collect model rows for one models.dev provider.
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


# Collect model rows for one cc-switch preset.
def collect_ccswitch(
    provider: SourceProvider,
    presets: dict[str, CcSwitchPreset],
    prices: dict[str, CcSwitchPricing],
) -> list[str]:
    preset = presets.get(provider.source_id)
    if preset is None:
        raise ValueError(f"ccswitch preset not found: {provider.source_id}")
    rows: list[str] = []
    for model in sorted(preset.models, key=lambda entry: entry.model.lower()):
        if model.context_window is None:
            continue
        price = ccswitch_pricing_for_model(prices, model.model)
        if price is None:
            continue
        image, audio, video = parsed_modality_flags(model.model, model.input_modalities)
        rows.append(
            model_row(
                provider.provider_type_id,
                model.model,
                price.input_price,
                price.cached_input_price,
                price.output_price,
                "USD",
                model.context_window,
                image,
                audio,
                video,
                model.supports_parallel_tool_calls,
            )
        )
    return rows


# Return the provider and model identity fields from one rendered row.
def model_row_key(model_row_data: str) -> tuple[str, str]:
    parts = model_row_data.split("|")
    if len(parts) != 16:
        raise ValueError(f"invalid model catalog row: {model_row_data}")
    return parts[0].casefold(), parts[1].casefold()


# Keep one row for each provider/model identity, using the latest source row.
def unique_model_rows(rows: list[str]) -> list[str]:
    row_order: list[tuple[str, str]] = []
    rows_by_key: dict[tuple[str, str], str] = {}
    for model_row_data in rows:
        key = model_row_key(model_row_data)
        if key not in rows_by_key:
            row_order.append(key)
        rows_by_key[key] = model_row_data
    return [rows_by_key[key] for key in row_order]


# Add rows whose provider/model identity is not already present.
def append_new_model_rows(target: list[str], incoming: list[str], existing_keys: set[tuple[str, str]]) -> None:
    for model_row_data in incoming:
        key = model_row_key(model_row_data)
        if key in existing_keys:
            continue
        existing_keys.add(key)
        target.append(model_row_data)


# Generate all provider model rows.
def generate_rows() -> list[str]:
    models_dev = required_object(fetch_json(MODELS_DEV_URL), "models.dev")
    litellm = required_object(fetch_json(LITELLM_PRICING_URL), "litellm")
    ccswitch_presets = parse_ccswitch_presets(fetch_text(CCSWITCH_CODEX_PROVIDER_PRESETS_URL))
    ccswitch_prices = parse_ccswitch_pricing(fetch_text(CCSWITCH_SCHEMA_URL))
    model_rows: list[str] = []
    for provider in PROVIDERS:
        if provider.source == "models.dev":
            rows = collect_models_dev(provider, models_dev)
        elif provider.source == "ccswitch":
            rows = collect_ccswitch(provider, ccswitch_presets, ccswitch_prices)
        else:
            raise ValueError(f"unknown source: {provider.source}")
        if len(rows) == 0:
            raise ValueError(f"{provider.provider_type_id} produced zero rows")
        model_rows.extend(rows)

    model_rows = unique_model_rows(model_rows)
    existing_keys = {model_row_key(model_row_data) for model_row_data in model_rows}
    for provider in LITELLM_HISTORY_PROVIDERS:
        rows = collect_litellm_history(provider, litellm)
        if len(rows) == 0:
            raise ValueError(f"{provider.provider_type_id} produced zero LiteLLM rows")
        append_new_model_rows(model_rows, rows, existing_keys)
    return model_rows


# Render model rows as a Rust constant.
def render(model_data: list[str]) -> str:
    model_block = "\n".join(model_data)
    return f"""pub const MODEL_CATALOG_MODEL_ROWS: &str = r#"
{model_block}
"#;
"""


# Run the generator command-line interface.
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

    with TARGET.open("w", encoding="utf-8", newline="\n") as output:
        output.write(generated)
    print(f"wrote {TARGET}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

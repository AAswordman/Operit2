#include "operit_local_llm_export.h"
#include "llama.h"
#include "llm/llm.hpp"
#include <nlohmann/json.hpp>
#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <mutex>
#include <stdexcept>
#include <string>
#include <vector>

using json = nlohmann::json;
namespace {
using Model = std::unique_ptr<llama_model, decltype(&llama_model_free)>;
using Context = std::unique_ptr<llama_context, decltype(&llama_free)>;
using Sampler = std::unique_ptr<llama_sampler, decltype(&llama_sampler_free)>;
std::mutex inference_mutex;
std::string loaded_key;
Model llama_model_cache{nullptr, llama_model_free};
std::unique_ptr<MNN::Transformer::Llm> mnn_cache;

std::string text_content(const json &message) {
    const auto &content = message.at("content");
    if (content.is_string()) return content.get<std::string>();
    if (content.is_null()) return "";
    std::string text;
    for (const auto &part : content) {
        if (part.value("type", "") != "text") throw std::runtime_error("Local text models do not support media input");
        text += part.at("text").get<std::string>();
    }
    return text;
}

json llama_generate(const std::string &path, const json &messages, const json &body, const json &settings) {
    static std::once_flag init;
    std::call_once(init, [] { llama_backend_init(); });
    if (!llama_model_cache) {
        auto parameters = llama_model_default_params();
        parameters.n_gpu_layers = 0; // CPU backend is available on every supported native platform.
        parameters.load_mode = settings.value("llamaUseMmap", true) ? LLAMA_LOAD_MODE_MMAP : LLAMA_LOAD_MODE_NONE;
        llama_model_cache.reset(llama_model_load_from_file(path.c_str(), parameters));
        if (!llama_model_cache) throw std::runtime_error("llama.cpp could not load the GGUF model");
    }
    auto *model = llama_model_cache.get();
    const auto *vocab = llama_model_get_vocab(model);
    auto parameters = llama_context_default_params();
    parameters.n_ctx = settings.value("llamaContextSize", 2048);
    parameters.n_batch = settings.value("llamaBatchSize", 512);
    parameters.n_ubatch = settings.value("llamaUBatchSize", 512);
    parameters.n_threads = settings.value("llamaThreadCount", 4);
    parameters.n_threads_batch = parameters.n_threads;
    parameters.flash_attn_type = settings.value("llamaFlashAttention", false) ? LLAMA_FLASH_ATTN_TYPE_ENABLED : LLAMA_FLASH_ATTN_TYPE_DISABLED;
    parameters.kv_unified = settings.value("llamaKvUnified", true);
    parameters.offload_kqv = false;
    Context context(llama_init_from_model(model, parameters), llama_free);
    if (!context) throw std::runtime_error("llama.cpp could not allocate the inference context");
    std::vector<std::string> roles, contents;
    for (const auto &message : messages) {
        roles.push_back(message.at("role").get<std::string>());
        contents.push_back(text_content(message));
    }
    std::vector<llama_chat_message> chat;
    for (size_t i = 0; i < roles.size(); ++i) chat.push_back({roles[i].c_str(), contents[i].c_str()});
    const char *chat_template = llama_model_chat_template(model, nullptr);
    if (!chat_template) throw std::runtime_error("GGUF model has no chat template; import an instruct/chat model");
    int length = llama_chat_apply_template(chat_template, chat.data(), chat.size(), true, nullptr, 0);
    if (length < 0) throw std::runtime_error("llama.cpp does not support this model's chat template");
    std::vector<char> prompt(length + 1);
    length = llama_chat_apply_template(chat_template, chat.data(), chat.size(), true, prompt.data(), prompt.size());
    if (length < 0 || static_cast<size_t>(length) >= prompt.size()) throw std::runtime_error("Chat template formatting failed");
    int count = llama_tokenize(vocab, prompt.data(), length, nullptr, 0, true, true);
    if (count >= 0) throw std::runtime_error("Chat prompt tokenization failed");
    std::vector<llama_token> tokens(-count);
    count = llama_tokenize(vocab, prompt.data(), length, tokens.data(), tokens.size(), true, true);
    if (count <= 0) throw std::runtime_error("Chat prompt is empty");
    const int maximum = body.value("max_tokens", 512);
    if (static_cast<uint32_t>(count) >= llama_n_ctx(context.get())) throw std::runtime_error("Prompt exceeds the configured local context size");
    int generated = 0;
    for (int offset = 0; offset < count;) {
        int size = std::min(count - offset, static_cast<int>(parameters.n_batch));
        if (llama_decode(context.get(), llama_batch_get_one(tokens.data() + offset, size))) throw std::runtime_error("llama.cpp prompt evaluation failed");
        offset += size;
    }
    Sampler sampler(llama_sampler_chain_init(llama_sampler_chain_default_params()), llama_sampler_free);
    const float temperature = body.value("temperature", 0.7f);
    if (temperature <= 0) llama_sampler_chain_add(sampler.get(), llama_sampler_init_greedy());
    else {
        llama_sampler_chain_add(sampler.get(), llama_sampler_init_top_k(body.value("top_k", 40)));
        llama_sampler_chain_add(sampler.get(), llama_sampler_init_top_p(body.value("top_p", 0.9f), 1));
        llama_sampler_chain_add(sampler.get(), llama_sampler_init_temp(temperature));
        llama_sampler_chain_add(sampler.get(), llama_sampler_init_dist(body.value("seed", LLAMA_DEFAULT_SEED)));
    }
    std::string output;
    auto stop = body.value("stop", json::array());
    if (stop.is_string()) stop = json::array({stop});
    for (; generated < maximum && count + generated < static_cast<int>(llama_n_ctx(context.get()));) {
        auto token = llama_sampler_sample(sampler.get(), context.get(), -1);
        if (llama_vocab_is_eog(vocab, token)) break;
        std::vector<char> piece(256);
        int size = llama_token_to_piece(vocab, token, piece.data(), piece.size(), 0, false);
        if (size < 0) { piece.resize(-size); size = llama_token_to_piece(vocab, token, piece.data(), piece.size(), 0, false); }
        if (size < 0) throw std::runtime_error("Token decoding failed");
        output.append(piece.data(), size);
        ++generated;
        bool stopped = false;
        if (stop.is_array()) for (const auto &item : stop) {
            if (!item.is_string() || item.get_ref<const std::string&>().empty()) continue;
            auto position = output.find(item.get_ref<const std::string&>());
            if (position != std::string::npos) { output.resize(position); stopped = true; break; }
        }
        if (stopped || generated == maximum) break;
        if (llama_decode(context.get(), llama_batch_get_one(&token, 1))) throw std::runtime_error("llama.cpp token evaluation failed");
    }
    return {{"text", output}, {"metadata", {{"engine", "llama.cpp"}, {"inputTokens", count}, {"outputTokens", generated}}}};
}

// MNN's loader can report success with missing embeddings. Validate the bundle
// before entering native loading and keep all generated caches on this device.
std::string prepare_mnn_config(const std::string &path) {
    namespace fs = std::filesystem;
    const auto root = fs::canonical(fs::path(path).parent_path());
    auto local_path = [&](const std::string &name, bool required) {
        const fs::path relative(name);
        if (relative.empty() || relative.is_absolute() || relative.has_root_name())
            throw std::runtime_error("MNN model paths must be relative to the installed bundle");
        for (const auto &part : relative) if (part == "..")
            throw std::runtime_error("MNN model path escapes its installed bundle");
        const auto resolved = fs::weakly_canonical(root / relative);
        const auto inside = resolved.lexically_relative(root);
        if (inside.empty() || *inside.begin() == "..")
            throw std::runtime_error("MNN model path escapes its installed bundle");
        if (required && (!fs::is_regular_file(resolved) || fs::file_size(resolved) == 0))
            throw std::runtime_error("MNN model file is missing or empty: " + name);
        return resolved;
    };
    auto read_json = [](const fs::path &file) {
        std::ifstream stream(file);
        if (!stream) throw std::runtime_error("Cannot read MNN configuration: " + file.string());
        auto value = json::parse(stream);
        if (!value.is_object()) throw std::runtime_error("MNN configuration must be an object");
        return value;
    };
    auto config = read_json(path);
    auto check_base = [](const json &value) {
        const auto base = value.value("base_dir", ".");
        if (base != "." && base != "./" && base != ".\\")
            throw std::runtime_error("MNN base_dir must refer to the installed bundle");
    };
    check_base(config);
    const auto model_config = read_json(local_path(config.value("llm_config", "llm_config.json"), true));
    check_base(model_config);
    config.update(model_config);
    if (config.value("is_visual", false) || config.value("is_audio", false) || config.value("has_talker", false))
        throw std::runtime_error("This local MNN provider supports text chat models");
    for (const auto &entry : {std::make_pair("llm_model", "llm.mnn"),
            std::make_pair("llm_weight", "llm.mnn.weight"), std::make_pair("tokenizer_file", "tokenizer.txt")})
        local_path(config.value(entry.first, entry.second), true);
    const auto tie = config.value("tie_embeddings", json());
    const bool tied = (tie.is_array() && tie.size() >= 5 && tie[0].is_number_integer() && tie[0].get<int64_t>() > 0)
        || (tie.is_object() && tie.value("weight_offset", int64_t(0)) > 0);
    if (!tied) local_path(config.value("embedding_file", "embeddings_bf16.bin"), true);
    else if (config.contains("embedding_file")) local_path(config.at("embedding_file").get<std::string>(), false);
    // New MNN tokenizers no longer apply the legacy per-role templates.
    // Preserve those bundles by translating their templates to Jinja once.
    if (!config.contains("jinja") || !config["jinja"].contains("chat_template")) {
        auto role_template = [&](const std::string &key, const std::string &fallback, bool prefix_only) {
            const auto text = config.value(key, fallback);
            const auto split = text.find("%s");
            if (split == std::string::npos) throw std::runtime_error("MNN role template requires a %s placeholder");
            const auto prefix = json(text.substr(0, split)).dump();
            if (prefix_only) return "{{ " + prefix + " }}";
            return "{{ " + prefix + " + message['content'] + " + json(text.substr(split + 2)).dump() + " }}";
        };
        std::string chat_template = "{% for message in messages %}{% if message['role'] == 'system' %}";
        chat_template += role_template("system_prompt_template", "<|im_start|>system\n%s<|im_end|>\n", false);
        chat_template += "{% elif message['role'] == 'assistant' %}";
        chat_template += role_template("assistant_prompt_template", "<|im_start|>assistant\n%s<|im_end|>\n", false);
        chat_template += "{% else %}";
        chat_template += role_template("user_prompt_template", "<|im_start|>user\n%s<|im_end|>\n", false);
        chat_template += "{% endif %}{% endfor %}{% if add_generation_prompt %}";
        chat_template += role_template("assistant_prompt_template", "<|im_start|>assistant\n%s<|im_end|>\n", true);
        chat_template += "{% endif %}";
        config["jinja"]["chat_template"] = chat_template;
    }
    // Flatten validated metadata into a private configuration. Remote repository
    // cache/base paths cannot redirect native writes outside this installation.
    const auto cache = local_path(".operit-runtime", false);
    fs::create_directories(cache);
    const auto runtime_config = local_path(".operit-runtime/config.json", false);
    const auto runtime_metadata = local_path(".operit-runtime/llm_config.json", false);
    config["base_dir"] = root.generic_string() + "/";
    config["llm_config"] = ".operit-runtime/llm_config.json";
    config["tmp_path"] = cache.generic_string();
    config["prefix_cache_path"] = cache.generic_string();
    config["backend_type"] = "cpu";
    config["use_mmap"] = false;
    config["use_cached_mmap"] = false;
    config["kvcache_mmap"] = false;
    config["reuse_kv"] = false;
    config["async"] = false;
    for (const auto &entry : {std::make_pair(runtime_metadata, std::string("{}")),
            std::make_pair(runtime_config, config.dump())}) {
        std::ofstream stream(entry.first, std::ios::trunc);
        if (!(stream << entry.second)) throw std::runtime_error("Cannot write local MNN runtime configuration");
    }
    return runtime_config.string();
}

json mnn_generate(const std::string &path, const json &messages, const json &body, const json &settings) {
    json generation = {{"sampler_type", body.value("temperature", 0.7) <= 0 ? "greedy" : "mixed"},
        {"max_new_tokens", body.value("max_tokens", 512)}, {"temperature", body.value("temperature", 0.7)},
        {"top_p", body.value("top_p", 0.9)}, {"top_k", body.value("top_k", 40)},
        {"jinja", {{"context", {{"enable_thinking", body.value("enable_thinking", false)}}}}}};
    if (!mnn_cache) {
        mnn_cache.reset(MNN::Transformer::Llm::createLLM(prepare_mnn_config(path)));
        if (!mnn_cache) throw std::runtime_error("MNN could not open the model configuration");
        json config = {{"backend_type", "cpu"}, {"thread_num", settings.value("mnnThreadCount", 4)}, {"async", false}};
        // MNN snapshots sampler options during load(), so apply them first.
        config.update(generation);
        if (!mnn_cache->set_config(config.dump()) || !mnn_cache->load()) { mnn_cache.reset(); throw std::runtime_error("MNN could not load the model"); }
    }
    mnn_cache->reset();
    if (!mnn_cache->set_config(generation.dump())) throw std::runtime_error("MNN rejected generation settings");
    MNN::Transformer::ChatMessages chat;
    for (const auto &message : messages) chat.emplace_back(message.at("role").get<std::string>(), text_content(message));
    const auto prompt = mnn_cache->apply_chat_template(chat);
    const auto tokens = mnn_cache->tokenizer_encode(prompt);
    const int limit = settings.value("llamaContextSize", 2048);
    if (tokens.empty() || tokens.size() >= static_cast<size_t>(limit))
        throw std::runtime_error("Prompt exceeds the configured local context size or is empty");
    std::ostringstream sink;
    mnn_cache->response(tokens, &sink, "", std::min(body.value("max_tokens", 512), limit - static_cast<int>(tokens.size())));
    const auto *context = mnn_cache->getContext();
    if (context->status == MNN::Transformer::LlmStatus::INTERNAL_ERROR || context->status == MNN::Transformer::LlmStatus::NOT_LOADED || context->status == MNN::Transformer::LlmStatus::TIMEOUT)
        throw std::runtime_error("MNN generation failed");
    auto output = context->generate_str;
    auto stop = body.value("stop", json::array());
    if (stop.is_string()) stop = json::array({stop});
    if (stop.is_array()) for (const auto &item : stop) {
        if (!item.is_string() || item.get_ref<const std::string&>().empty()) continue;
        const auto position = output.find(item.get_ref<const std::string&>());
        if (position != std::string::npos) output.resize(position);
    }
    return {{"text", output}, {"metadata", {{"engine", "MNN"}, {"inputTokens", context->prompt_len}, {"outputTokens", context->gen_seq_len}}}};
}

} // namespace

/// Generates one response and keeps C++ exception ownership inside the engine.
extern "C" OPERIT_LOCAL_LLM_EXPORT char *operit_local_llm_generate(const char *engine, const char *path, const char *request_json, const char *settings_json) noexcept {
    std::lock_guard<std::mutex> lock(inference_mutex);
    json result;
    try {
        const auto body = json::parse(request_json);
        const auto settings = json::parse(settings_json);
        std::string key = std::string(engine) + ":" + path + ":" + settings_json;
        if (std::strcmp(engine, "mnn") == 0) key += json({{"temperature", body.value("temperature", 0.7)},
            {"top_p", body.value("top_p", 0.9)}, {"top_k", body.value("top_k", 40)}}).dump();
        if (loaded_key != key) { mnn_cache.reset(); llama_model_cache.reset(); loaded_key = key; }
        if (std::strcmp(engine, "llama-cpp") == 0) result = llama_generate(path, body.at("messages"), body, settings);
        else if (std::strcmp(engine, "mnn") == 0) result = mnn_generate(path, body.at("messages"), body, settings);
        else throw std::runtime_error("Unknown local LLM engine");
    } catch (const std::exception &error) { result = {{"error", error.what()}}; }
    catch (...) { result = {{"error", "Native local inference failed"}}; }
    const auto text = result.dump(-1, ' ', false, json::error_handler_t::replace);
    auto *output = static_cast<char *>(std::malloc(text.size() + 1));
    if (output) std::memcpy(output, text.c_str(), text.size() + 1);
    return output;
}
/// Releases response storage in the same runtime that allocated it.
extern "C" OPERIT_LOCAL_LLM_EXPORT void operit_local_llm_free(char *text) noexcept { std::free(text); }
/// Releases the engine-owned model caches across the plain C boundary.
extern "C" OPERIT_LOCAL_LLM_EXPORT void operit_local_llm_unload() noexcept {
    std::lock_guard<std::mutex> lock(inference_mutex);
    mnn_cache.reset(); llama_model_cache.reset(); loaded_key.clear();
}

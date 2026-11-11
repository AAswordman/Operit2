#pragma once

#include <stdexcept>
#include <string>
#include <vector>

namespace operit {
struct SparkChatMessage {
    std::string role;
    std::string content;
    std::string reasoningContent;
};

// Spark X2.5/X2.6 has its own role tokens. The pinned llama.cpp chat formatter
// recognizes the architecture but does not recognize this Jinja template.
inline bool isSparkChatTemplate(const std::string &chatTemplate) {
    return chatTemplate.find("<｜start▁of▁sentence｜>") != std::string::npos
        && chatTemplate.find("<|System|>") != std::string::npos
        && chatTemplate.find("<|Bot|>") != std::string::npos;
}

// Text-only rendering of XHToken/Spark-X2.6-1.7B's published tokenizer template.
// Keep historical assistant reasoning and group consecutive tool responses as
// the model expects. Tool definitions/calls are outside this text driver.
inline std::string formatSparkChatPrompt(
    const std::vector<SparkChatMessage> &messages,
    bool enableThinking,
    const std::string &reasoningEffort = "high") {
    if (messages.empty()) throw std::runtime_error("Local chat requires messages");
    const std::string start = "<｜start▁of▁sentence｜>";
    const std::string end = "<｜end▁of▁sentence｜>";
    std::string prompt = start + "<|System|>\nyou are a helpful assistant.";
    if (enableThinking) {
        if (reasoningEffort != "low" && reasoningEffort != "high") {
            throw std::runtime_error("Spark reasoning_effort must be low or high");
        }
        prompt += "\n`thinking_effort` guides on how much to think in your thinking channel "
            "(not including the response channel), supported values include `low` and `high`."
            "\nNow the system is invoked with `thinking_effort=" + reasoningEffort + "`.";
    }
    if (messages.front().role == "system" && !messages.front().content.empty()) {
        prompt += "\n\n" + messages.front().content;
    }
    prompt += end;
    for (size_t i = 0; i < messages.size(); ++i) {
        const auto &message = messages[i];
        if (message.role == "system") {
            if (i != 0) prompt += start + "<|System|>\n" + message.content + end;
        } else if (message.role == "user") {
            prompt += start + "<|User|>" + message.content + end;
        } else if (message.role == "assistant") {
            prompt += start + "<|Bot|>";
            if (!message.reasoningContent.empty()) {
                prompt += "<think>" + message.reasoningContent;
            }
            prompt += "</think>" + message.content + end;
        } else if (message.role == "tool") {
            if (i == 0 || messages[i - 1].role != "tool") prompt += start + "<|Tool|>";
            prompt += "<tool_response>" + message.content + "</tool_response>";
            if (i + 1 == messages.size() || messages[i + 1].role != "tool") prompt += end;
        } else {
            throw std::runtime_error("Unsupported Spark message role: " + message.role);
        }
    }
    return prompt + start + "<|Bot|>" + (enableThinking ? "<think>" : "</think>");
}
} // namespace operit

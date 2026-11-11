#include "../SparkChatTemplate.hpp"
#include <cassert>
#include <iostream>

int main() {
    using operit::formatSparkChatPrompt;
    const std::string start = "<｜start▁of▁sentence｜>";
    const std::string end = "<｜end▁of▁sentence｜>";
    const std::string system = start + "<|System|>\nyou are a helpful assistant.";

    assert(operit::isSparkChatTemplate(start + "<|System|><|Bot|>"));
    assert(!operit::isSparkChatTemplate("<|im_start|>user\nHello<|im_end|>"));
    assert(!operit::isSparkChatTemplate(start + "<｜User｜><｜Assistant｜>"));

    assert(formatSparkChatPrompt({{"user", "你好", ""}}, false)
        == system + end + start + "<|User|>你好" + end + start + "<|Bot|></think>");

    assert(formatSparkChatPrompt({
        {"system", "用中文回答", ""}, {"user", "1+1?", ""},
        {"assistant", "2", "相加"}, {"system", "简短", ""},
        {"tool", "one", ""}, {"tool", "two", ""}, {"user", "继续", ""},
    }, false) == system + "\n\n用中文回答" + end
        + start + "<|User|>1+1?" + end
        + start + "<|Bot|><think>相加</think>2" + end
        + start + "<|System|>\n简短" + end
        + start + "<|Tool|><tool_response>one</tool_response><tool_response>two</tool_response>" + end
        + start + "<|User|>继续" + end + start + "<|Bot|></think>");

    assert(formatSparkChatPrompt({{"assistant", "answer", ""}}, false)
        == system + end + start + "<|Bot|></think>answer" + end + start + "<|Bot|></think>");

    for (const auto &effort : {"low", "high"}) {
        const auto prompt = formatSparkChatPrompt({{"user", "think", ""}}, true, effort);
        assert(prompt == system + "\n`thinking_effort` guides on how much to think in your thinking channel "
            "(not including the response channel), supported values include `low` and `high`."
            "\nNow the system is invoked with `thinking_effort=" + effort + "`." + end
            + start + "<|User|>think" + end + start + "<|Bot|><think>");
    }
    bool rejected = false;
    try { formatSparkChatPrompt({{"user", "test", ""}}, true, "invalid"); }
    catch (const std::runtime_error &) { rejected = true; }
    assert(rejected);
    std::cout << "Spark chat template tests passed\n";
}

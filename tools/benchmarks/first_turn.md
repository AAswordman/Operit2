# First-response prompt inspection

`first_turn.py` sends one saved Chat Completions request to a configured provider.
It records streamed reasoning, visible text, proposed tool calls, provider usage,
finish reason and timing. It never executes proposed tools or continues the agent
loop. It does not build or launch the CLI.

```sh
python tools/benchmarks/first_turn.py \
  --request /path/to/experiment.request.json \
  --providers /path/to/providers.json \
  --provider DEEPSEEK \
  --output /path/to/results/experiment
```

The provider file uses the CLI providers-export/import list format. Credentials
are read only to construct the HTTP headers; request headers are not saved.
The request must enable streaming and should include
`"stream_options": {"include_usage": true}` and an explicit output token budget.
Keep the model, reasoning settings, task and tool schemas fixed for prompt-only
comparisons. Give repetitions different output prefixes; existing results are
never overwritten and failed requests are not retried. Partial results survive
a transport or streaming error.

Outputs are `PREFIX.response.json`, `PREFIX.thinking.txt` and
`PREFIX.output.txt`. Keep the original request bodies next to the results so
comparisons are reviewable. `finish_reason: "length"` indicates a truncated
response, not successful completion. Cache hits affect measured cost and latency.

An initial response can reveal premature assumptions, guessed tool names and
excessive planning. It does not establish implementation correctness, later tool
behavior, testing quality or when the complete agent loop would stop. Describe
reconstructed or adapted harness inputs explicitly; a different agent's role
text with Operit tools is a controlled prompt experiment, not a native-agent run.

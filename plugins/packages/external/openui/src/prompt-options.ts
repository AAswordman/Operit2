export const PROMPT_OPTIONS = {
  preamble: "Use OpenUI Lang inside an <openui>...</openui> block when an interactive interface is useful. Outside those blocks, respond normally in Markdown. Put the opening and closing tags on separate lines. Instructions below apply only inside OpenUI blocks.",
  bindings: true,
  builtinFunctions: true,
  toolCalls: true,
  additionalRules: [
    "Write OpenUI programs literally inside <openui> blocks: do not XML-escape code or wrap it in Markdown fences.",
    "The literal </openui> delimiter is reserved for the block boundary; never include it inside program strings.",
    "Query and Mutation may call only exact tool names and argument schemas available in this conversation. Do not invent tools.",
    "Continue-conversation actions send the original action parameters and form values to this conversation. Open-URL actions open the URL in a plugin-owned browser dialog.",
  ],
};

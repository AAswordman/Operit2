import type { ComposeDslContext, ComposeNode } from "../../../../../../types/compose-dsl";
import { parsePlanaskXml } from "../../shared/plan_mode_ask.js";
import { submitPlanaskAnswers } from "../../shared/plan_mode_ask_execution.js";
import { resolvePlanModeI18n } from "../../shared/plan_mode_i18n.js";

const LOG_TAG = "[plan_mode_planask]";

/** Wraps a Compose state pair in named value and setter fields. */
function useStateValue<T>(ctx: ComposeDslContext, key: string, initialValue: T): {
  value: T;
  set: (value: T) => void;
} {
  const pair = ctx.useState<T>(key, initialValue);
  return { value: pair[0], set: pair[1] };
}

/** Clips a value for bounded diagnostic logging. */
function clipLogText(value: string, maxLength = 80): string {
  const normalized = String(value || "").replace(/\n/g, "\\n");
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength)}...`;
}

/** Writes per-answer details at the shared V6 diagnostic depth. */
function logPlanaskVerbose(message: string, payload?: Record<string, unknown>): void {
  if (!payload) {
    console.verbose(6, `${LOG_TAG} ${message}`);
    return;
  }
  console.verbose(6, `${LOG_TAG} ${message} ${JSON.stringify(payload)}`);
}

/** Tests whether a question has an effective selected or custom answer. */
function isQuestionAnswered(
  question: { id: string; options: Array<{ id: string; label: string }> },
  answers: Record<string, string>,
  customAnswers: Record<string, string>
): boolean {
  return resolveAnswerText(question, answers, customAnswers).trim() !== "";
}

/** Resolves the visible answer text from the question state. */
function resolveAnswerText(
  question: { id: string; options: Array<{ id: string; label: string }> },
  selectedAnswers: Record<string, string>,
  customAnswers: Record<string, string>
): string {
  const customText = (customAnswers[question.id] ?? "").trim();
  if (customText !== "") {
    return customText;
  }
  const selectedOptionId = selectedAnswers[question.id];
  const selectedOption = question.options.find((option) => option.id === selectedOptionId);
  return selectedOption ? selectedOption.label : "";
}

/** Builds the message submitted to the active chat from UI answer state. */
function buildPlanaskAnswerMessageLocal(
  parsed: {
    title: string;
    questions: Array<{ id: string; title: string }>;
  },
  answerTexts: Record<string, string>
): string {
  const selectedQuestions = parsed.questions
    .map((question) => {
      const answerText = String(answerTexts[question.id] || "").trim();
      if (!answerText) {
        return "";
      }
      return `- ${question.title}：${answerText}`;
    })
    .filter((item) => item !== "");

  const lines = ["计划确认答复："];
  if (parsed.title) {
    lines.push(`主题：${parsed.title}`);
  }
  selectedQuestions.forEach((item) => lines.push(item));
  return lines.join("\n");
}

/** Renders streamed plan clarification questions and answer submission controls. */
export default function Screen(ctx: ComposeDslContext): ComposeNode {
  const text = resolvePlanModeI18n();
  const [xmlContent] = ctx.useState("xmlContent", "");
  const [chatId] = ctx.useState<string | null>("chatId", null);
  const answersState = useStateValue<Record<string, string>>(ctx, "answers", {});
  const customAnswersState = useStateValue<Record<string, string>>(ctx, "customAnswers", {});
  const questionIndexState = useStateValue(ctx, "questionIndex", 0);
  const submittingState = useStateValue(ctx, "submitting", false);
  const submittedState = useStateValue(ctx, "submitted", false);
  const errorState = useStateValue(ctx, "error", "");
  const closedState = useStateValue(ctx, "closed", false);

  const parsed = parsePlanaskXml(xmlContent);
  const ready = parsed.closed || closedState.value;
  const currentQuestionIndex =
    parsed.questions.length === 0
      ? 0
      : Math.min(questionIndexState.value, parsed.questions.length - 1);
  const currentQuestion =
    parsed.questions.length > 0 ? parsed.questions[currentQuestionIndex] : undefined;
  const allAnswered =
    parsed.questions.length > 0 &&
    parsed.questions.every(
      (question) => isQuestionAnswered(question, answersState.value, customAnswersState.value)
    );

  /** Stores a single selected option and clears any custom response. */
  const handleSelect = (questionId: string, optionId: string): void => {
    const nextAnswers = {
      ...answersState.value,
      [questionId]: optionId,
    };
    const nextCustomAnswers = { ...customAnswersState.value };
    delete nextCustomAnswers[questionId];
    answersState.set(nextAnswers);
    customAnswersState.set(nextCustomAnswers);
    if (errorState.value !== "") {
      errorState.set("");
    }
  };

  /** Stores a custom response and synchronizes its selection marker. */
  const handleCustomAnswerChange = (questionId: string, value: string): void => {
    const nextCustomAnswers = { ...customAnswersState.value };
    if (value !== "") {
      nextCustomAnswers[questionId] = value;
    } else {
      delete nextCustomAnswers[questionId];
    }
    const nextAnswers = { ...answersState.value };
    if (value.trim() !== "") {
      nextAnswers[questionId] = "__custom__";
    } else if (nextAnswers[questionId] === "__custom__") {
      delete nextAnswers[questionId];
    }
    logPlanaskVerbose("custom_answer_change", {
      questionId,
      valueLength: value.length,
      valuePreview: clipLogText(value),
      storedValueLength: (nextCustomAnswers[questionId] ?? "").length,
      storedValuePreview: clipLogText(nextCustomAnswers[questionId] ?? ""),
      selectedAnswerMarker: nextAnswers[questionId] || "",
      storedCustomAnswerKeys: Object.keys(nextCustomAnswers),
    });
    customAnswersState.set(nextCustomAnswers);
    answersState.set(nextAnswers);
    if (errorState.value !== "") {
      errorState.set("");
    }
  };

  /** Selects the preceding clarification question. */
  const goToPreviousQuestion = (): void => {
    const nextIndex = Math.max(0, currentQuestionIndex - 1);
    questionIndexState.set(nextIndex);
  };

  /** Selects the following clarification question. */
  const goToNextQuestion = (): void => {
    const nextIndex = Math.min(parsed.questions.length - 1, currentQuestionIndex + 1);
    questionIndexState.set(nextIndex);
  };

  /** Validates and submits the resolved clarification answers. */
  const handleSubmit = async (): Promise<void> => {
    if (submittingState.value || submittedState.value) {
      return;
    }
    if (!ready || !allAnswered) {
      errorState.set(text.askRendererSelectRequired);
      await ctx.showToast(text.askRendererSelectRequired);
      return;
    }
    if (chatId === null || chatId.trim() === "") {
      errorState.set(text.toastChatViewMissing);
      await ctx.showToast(text.toastChatViewMissing);
      return;
    }
    errorState.set("");
    submittingState.set(true);
    const answerTexts: Record<string, string> = {};
    parsed.questions.forEach((question) => {
      answerTexts[question.id] = resolveAnswerText(
        question,
        answersState.value,
        customAnswersState.value
      );
    });
    logPlanaskVerbose("submit_answers", {
      answers: parsed.questions.map((question) => ({
        questionId: question.id,
        answerLength: (answerTexts[question.id] ?? "").length,
        answerPreview: clipLogText(answerTexts[question.id] ?? ""),
      })),
    });
    const message = buildPlanaskAnswerMessageLocal(parsed, answerTexts);
    const result = await submitPlanaskAnswers(message, chatId);
    submittingState.set(false);
    if (result.success) {
      submittedState.set(true);
      return;
    }
    errorState.set(result.error ?? "");
  };

  const headerTitle = parsed.title || text.askRendererTitle;
  const description = parsed.description || text.askRendererDescriptionFallback;
  const questionNode: ComposeNode | null = currentQuestion
    ? ctx.UI.Card(
        {
          key: `planask-question-${currentQuestion.id}`,
          fillMaxWidth: true,
          containerColor: ctx.MaterialTheme.colorScheme.surfaceVariant.copy({ alpha: 0.22 }),
          shape: { cornerRadius: 8 },
          elevation: 0,
        },
        [
          ctx.UI.Column(
            {
              fillMaxWidth: true,
              padding: 12,
              spacing: 10,
            },
            [
              ctx.UI.Row(
                {
                  fillMaxWidth: true,
                  horizontalArrangement: "spaceBetween",
                  verticalAlignment: "center",
                },
                [
                  ctx.UI.Text({
                    text: `${text.askRendererQuestionPrefix} ${currentQuestionIndex + 1}`,
                    style: "labelMedium",
                    color: "primary",
                    fontWeight: "bold",
                  }),
                  parsed.questions.length > 1
                    ? ctx.UI.Row(
                        {
                          verticalAlignment: "center",
                          horizontalArrangement: "end",
                          spacing: 0,
                        },
                        [
                          ctx.UI.IconButton({
                            enabled: currentQuestionIndex > 0,
                            icon: "chevronLeft",
                            onClick: goToPreviousQuestion,
                          }),
                          ctx.UI.Text({
                            text: `${currentQuestionIndex + 1}/${parsed.questions.length}`,
                            style: "bodySmall",
                            color: "onSurfaceVariant",
                            fontWeight: "medium",
                          }),
                          ctx.UI.IconButton({
                            enabled: currentQuestionIndex < parsed.questions.length - 1,
                            icon: "chevronRight",
                            onClick: goToNextQuestion,
                          }),
                        ]
                      )
                    : ctx.UI.Text({
                        text: "1/1",
                        style: "labelMedium",
                        color: "onSurfaceVariant",
                        fontWeight: "medium",
                      }),
                ]
              ),
              ctx.UI.Text({
                text: currentQuestion.title,
                style: "titleSmall",
                color: "onSurface",
                fontWeight: "semibold",
              }),
              ctx.UI.Column(
                {
                  fillMaxWidth: true,
                  spacing: 8,
                },
                currentQuestion.options.map((option) => {
                  const isSelected = answersState.value[currentQuestion.id] === option.id;
                  const optionContent = [
                    ctx.UI.Row(
                      {
                        fillMaxWidth: true,
                        horizontalArrangement: "start",
                        verticalAlignment: "center",
                      },
                      [
                        ctx.UI.Text({
                          text: option.label,
                          style: "bodyMedium",
                          color: isSelected ? "onPrimaryContainer" : "onSurface",
                          fontWeight: isSelected ? "semibold" : "medium",
                        }),
                      ]
                    ),
                  ];

                  return isSelected
                    ? ctx.UI.FilledTonalButton(
                        {
                          key: `planask-option-${currentQuestion.id}-${option.id}`,
                          fillMaxWidth: true,
                          containerColor: "primaryContainer",
                          contentColor: "onPrimaryContainer",
                          onClick: () => {
                            handleSelect(currentQuestion.id, option.id);
                          },
                          shape: { cornerRadius: 10 },
                          contentPadding: { horizontal: 14, vertical: 12 },
                        },
                        optionContent
                      )
                    : ctx.UI.OutlinedButton(
                        {
                          key: `planask-option-${currentQuestion.id}-${option.id}`,
                          fillMaxWidth: true,
                          onClick: () => {
                            handleSelect(currentQuestion.id, option.id);
                          },
                          shape: { cornerRadius: 10 },
                          contentPadding: { horizontal: 14, vertical: 12 },
                        },
                        optionContent
                      );
                })
              ),
              ctx.UI.TextField({
                key: `planask-custom-input-${currentQuestion.id}`,
                fillMaxWidth: true,
                label: text.askRendererCustomFieldLabel,
                placeholder: text.askRendererCustomFieldPlaceholder,
                value: customAnswersState.value[currentQuestion.id] ?? "",
                onValueChange: (value) => {
                  handleCustomAnswerChange(currentQuestion.id, value);
                },
                singleLine: false,
                enabled: !submittedState.value,
              }),
            ]
          ),
        ]
      )
    : null;

  const children: ComposeNode[] = [
    ctx.UI.Card(
      {
        fillMaxWidth: true,
        paddingHorizontal: 4,
        paddingVertical: 2,
        containerColor: ctx.MaterialTheme.colorScheme.surface.copy({ alpha: 0.95 }),
        shape: { cornerRadius: 8 },
        elevation: 2,
      },
        [
          ctx.UI.Column(
            {
              fillMaxWidth: true,
              padding: 16,
              spacing: 12,
            },
            [
              ctx.UI.Row({ verticalAlignment: "center", spacing: 10 }, [
                ctx.UI.Icon({ name: "quiz", tint: "primary", size: 22 }),
                ctx.UI.Column({ spacing: 2, weight: 1 }, [
                  ctx.UI.Text({
                    text: text.askRendererTitle,
                    style: "titleSmall",
                    fontWeight: "semibold",
                    color: ctx.MaterialTheme.colorScheme.onSurface,
                    fontSize: 13,
                  }),
                ]),
              ]),
              ctx.UI.Text({
                text: ready ? headerTitle : text.askRendererTitle,
                style: "headlineSmall",
                color: "onSurface",
                fontWeight: "bold",
              }),
              ...(ready
                ? [
                    ctx.UI.Markdown({
                      text: description,
                      color: "onSurfaceVariant",
                      fontSize: 12,
                      fillMaxWidth: true,
                      padding: { vertical: 2 },
                    }),
                    ...(questionNode
                      ? [questionNode]
                      : [
                          ctx.UI.Text({
                            text: text.rendererEmpty,
                            style: "bodyMedium",
                            color: "onSurfaceVariant",
                          }),
                        ]),
                  ]
                : [
                    ctx.UI.Row(
                      {
                        fillMaxWidth: true,
                        verticalAlignment: "center",
                        spacing: 10,
                      },
                      [
                        ctx.UI.CircularProgressIndicator({
                          width: 16,
                          height: 16,
                          strokeWidth: 2,
                          color: "primary",
                        }),
                        ctx.UI.Text({
                          text: text.askRendererStreamingHint,
                          style: "bodySmall",
                          color: "onSurfaceVariant",
                        }),
                      ]
                    ),
                  ]),
              ...(ready
                ? [
                    ctx.UI.Row(
                      {
                        fillMaxWidth: true,
                        horizontalArrangement: "end",
                      },
                      [
                        submittingState.value
                          ? ctx.UI.Button(
                              {
                                enabled: false,
                                onClick: handleSubmit,
                                contentPadding: { horizontal: 12, vertical: 8 },
                              },
                              [
                                ctx.UI.Row(
                                  {
                                    verticalAlignment: "center",
                                    horizontalArrangement: "center",
                                    spacing: 8,
                                  },
                                  [
                                    ctx.UI.CircularProgressIndicator({
                                      width: 14,
                                      height: 14,
                                      strokeWidth: 2,
                                      color: "onPrimary",
                                    }),
                                    ctx.UI.Text({ text: text.askRendererSubmitBusy }),
                                  ]
                                ),
                              ]
                            )
                          : ctx.UI.Button({
                              text: submittedState.value
                                ? text.askRendererSubmitted
                                : text.askRendererSubmitIdle,
                              enabled: allAnswered && !submittedState.value,
                              onClick: handleSubmit,
                              contentPadding: { horizontal: 12, vertical: 8 },
                            }),
                      ]
                    ),
                  ]
                : []),
          ]
        ),
      ]
    ),
  ];

  if (errorState.value !== "") {
    children.push(
      ctx.UI.Card(
        {
          fillMaxWidth: true,
          containerColor: "errorContainer",
          shape: { cornerRadius: 12 },
          elevation: 0,
        },
        [
          ctx.UI.Row(
            {
              padding: { horizontal: 14, vertical: 12 },
              spacing: 8,
              verticalAlignment: "center",
            },
            [
              ctx.UI.Icon({ name: "error", tint: "onErrorContainer", size: 18 }),
              ctx.UI.Text({
                text: errorState.value,
                style: "bodyMedium",
                color: "onErrorContainer",
              }),
            ]
          ),
        ]
      )
    );
  }

  return ctx.UI.Column(
    {
      key: ready ? "planask-ready" : "planask-streaming",
      fillMaxWidth: true,
      spacing: 12,
      onLoad: () => {
        if (parsed.closed && !closedState.value) {
          closedState.set(true);
        }
      },
    },
    children
  );
}

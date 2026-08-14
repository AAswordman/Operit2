import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import {
  createDefaultSettings,
  loadSettings,
  MAX_INJECTION_TIMEOUT_SECONDS,
  MIN_INJECTION_TIMEOUT_SECONDS,
  resolveExtraInfoI18n,
  saveSettings,
  type ExtraInfoInjectionSettings,
} from "../shared";

function useStateValue<T>(ctx: ComposeDslContext, key: string, initialValue: T): {
  value: T;
  set: (value: T) => void;
} {
  const pair = ctx.useState<T>(key, initialValue);
  return { value: pair[0], set: pair[1] };
}

async function readSettings(): Promise<ExtraInfoInjectionSettings> {
  return loadSettings();
}

async function saveSettingsInMain(
  patch: Partial<ExtraInfoInjectionSettings>
): Promise<ExtraInfoInjectionSettings> {
  return withContext("main", { patch }, () => saveSettings(patch));
}

function createSectionTitle(ctx: ComposeDslContext, icon: string, title: string): ComposeNode {
  return ctx.UI.Row({ verticalAlignment: "center" }, [
    ctx.UI.Icon({ name: icon, tint: "primary", size: 20 }),
    ctx.UI.Spacer({ width: 8 }),
    ctx.UI.Text({
      text: title,
      style: "titleMedium",
      fontWeight: "bold",
      color: "primary",
    }),
  ]);
}

const toggleCardStyle = {
  fillMaxWidth: true,
  shape: { cornerRadius: 8 },
  containerColor: "surfaceVariant",
  alpha: 0.38,
} as const;

function createDivider(ctx: ComposeDslContext): ComposeNode {
  return ctx.UI.HorizontalDivider({
    padding: { horizontal: 14 },
    color: "outlineVariant",
    thickness: 1,
  });
}

function createToggleCard(
  ctx: ComposeDslContext,
  title: string,
  subtitle: string,
  checked: boolean,
  onCheckedChange: (checked: boolean) => void,
  enabled = true
): ComposeNode {
  return ctx.UI.Surface(toggleCardStyle, [
    createToggleRow(ctx, title, subtitle, checked, onCheckedChange, enabled),
  ]);
}

function createToggleRow(
  ctx: ComposeDslContext,
  title: string,
  subtitle: string,
  checked: boolean,
  onCheckedChange: (checked: boolean) => void,
  enabled = true
): ComposeNode {
  return ctx.UI.Row(
    {
      fillMaxWidth: true,
      padding: { horizontal: 14, vertical: 12 },
      verticalAlignment: "center",
      horizontalArrangement: "spaceBetween",
    },
    [
      ctx.UI.Column({ weight: 1, spacing: 4 }, [
        ctx.UI.Text({
          text: title,
          style: "bodyMedium",
          fontWeight: "medium",
        }),
        ctx.UI.Text({
          text: subtitle,
          style: "bodySmall",
          color: "onSurfaceVariant",
        }),
      ]),
      ctx.UI.Spacer({ width: 12 }),
      ctx.UI.Switch({
        checked,
        enabled,
        onCheckedChange,
      }),
    ]
  );
}

type ToggleCardItem = {
  title: string;
  subtitle: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  enabled?: boolean;
};

function createMemoryConfigSection(
  ctx: ComposeDslContext,
  text: ReturnType<typeof resolveExtraInfoI18n>,
  enabled: boolean,
  allowRepeatedMemorySearch: boolean,
  onAllowRepeatedMemorySearchChange: (checked: boolean) => void,
  limitValue: string,
  onLimitChange: (value: string) => void,
  onApply: () => void
): ComposeNode {
  return ctx.UI.Column(
    {
      fillMaxWidth: true,
      padding: { horizontal: 14, vertical: 12 },
      spacing: 10,
    },
    [
      ctx.UI.Text({
        text: text.memoryConfigTitle,
        style: "bodyMedium",
        fontWeight: "medium",
      }),
      ctx.UI.Text({
        text: text.memoryConfigDescription,
        style: "bodySmall",
        color: "onSurfaceVariant",
      }),
      createToggleRow(
        ctx,
        text.memoryRepeatToggleTitle,
        text.memoryRepeatToggleDescription,
        allowRepeatedMemorySearch,
        onAllowRepeatedMemorySearchChange,
        enabled
      ),
      createDivider(ctx),
      ctx.UI.TextField({
        enabled,
        label: text.memoryLimitFieldLabel,
        placeholder: text.memoryLimitFieldPlaceholder,
        value: limitValue,
        onValueChange: onLimitChange,
        singleLine: true,
      }),
      ctx.UI.Text({
        text: text.memoryLimitFieldDescription,
        style: "bodySmall",
        color: "onSurfaceVariant",
      }),
      ctx.UI.Button({
        text: text.memoryConfigApplyButton,
        enabled,
        fillMaxWidth: true,
        onClick: onApply,
      }),
    ]
  );
}

/// Builds the configurable total deadline for one extra-info collection round.
function createInjectionTimeoutConfigSection(
  ctx: ComposeDslContext,
  text: ReturnType<typeof resolveExtraInfoI18n>,
  value: string,
  onValueChange: (value: string) => void,
  onApply: () => void
): ComposeNode {
  return ctx.UI.Surface(toggleCardStyle, [
    ctx.UI.Column(
      { fillMaxWidth: true, padding: { horizontal: 14, vertical: 12 }, spacing: 10 },
      [
        ctx.UI.TextField({
          label: text.injectionTimeoutFieldLabel,
          placeholder: text.injectionTimeoutFieldPlaceholder,
          value,
          onValueChange,
          singleLine: true,
        }),
        ctx.UI.Text({
          text: text.injectionTimeoutFieldDescription,
          style: "bodySmall",
          color: "onSurfaceVariant",
        }),
        ctx.UI.Button({
          text: text.injectionTimeoutApplyButton,
          fillMaxWidth: true,
          onClick: onApply,
        }),
      ]
    ),
  ]);
}

function createInjectionItemsCard(
  ctx: ComposeDslContext,
  items: ToggleCardItem[],
  memoryConfigSection: ComposeNode
): ComposeNode {
  const children: ComposeNode[] = [];

  items.forEach((item, index) => {
    children.push(
      createToggleRow(
        ctx,
        item.title,
        item.subtitle,
        item.checked,
        item.onCheckedChange,
        item.enabled ?? true
      )
    );
    children.push(createDivider(ctx));

    if (index === items.length - 1) {
      children.push(memoryConfigSection);
    }
  });

  return ctx.UI.Surface(toggleCardStyle, [
    ctx.UI.Column({ fillMaxWidth: true }, children),
  ]);
}

export default function Screen(ctx: ComposeDslContext): ComposeNode {
  const text = resolveExtraInfoI18n();
  const initial = createDefaultSettings();

  const masterEnabledState = useStateValue(ctx, "masterEnabled", initial.masterEnabled);
  const persistInjectedContentState = useStateValue(
    ctx,
    "persistInjectedContent",
    initial.persistInjectedContent
  );
  const injectTimeState = useStateValue(ctx, "injectTime", initial.injectTime);
  const injectBatteryState = useStateValue(ctx, "injectBattery", initial.injectBattery);
  const injectWeatherState = useStateValue(ctx, "injectWeather", initial.injectWeather);
  const injectLocationState = useStateValue(ctx, "injectLocation", initial.injectLocation);
  const usePreciseLocationState = useStateValue(
    ctx,
    "usePreciseLocation",
    initial.usePreciseLocation
  );
  const injectCurrentScreenAppState = useStateValue(
    ctx,
    "injectCurrentScreenApp",
    initial.injectCurrentScreenApp
  );
  const injectRecentAppUsageState = useStateValue(
    ctx,
    "injectRecentAppUsage",
    initial.injectRecentAppUsage
  );
  const injectScreenTextState = useStateValue(ctx, "injectScreenText", initial.injectScreenText);
  const injectNotificationsState = useStateValue(
    ctx,
    "injectNotifications",
    initial.injectNotifications
  );
  const injectMemoryState = useStateValue(ctx, "injectMemory", initial.injectMemory);
  const allowRepeatedMemorySearchState = useStateValue(
    ctx,
    "allowRepeatedMemorySearch",
    initial.allowRepeatedMemorySearch
  );
  const memoryLimitState = useStateValue(ctx, "memoryLimit", initial.memoryLimit);
  const memoryLimitInputState = useStateValue(
    ctx,
    "memoryLimitInput",
    String(initial.memoryLimit)
  );
  const injectionTimeoutSecondsState = useStateValue(
    ctx,
    "injectionTimeoutSeconds",
    initial.injectionTimeoutSeconds
  );
  const injectionTimeoutInputState = useStateValue(
    ctx,
    "injectionTimeoutInput",
    String(initial.injectionTimeoutSeconds)
  );
  const successMessageState = useStateValue(ctx, "successMessage", "");
  const errorMessageState = useStateValue(ctx, "errorMessage", "");
  const hasInitializedState = useStateValue(ctx, "hasInitialized", false);

  const syncSettings = (next: ExtraInfoInjectionSettings): void => {
    masterEnabledState.set(next.masterEnabled);
    persistInjectedContentState.set(next.persistInjectedContent);
    injectTimeState.set(next.injectTime);
    injectBatteryState.set(next.injectBattery);
    injectWeatherState.set(next.injectWeather);
    injectLocationState.set(next.injectLocation);
    usePreciseLocationState.set(next.usePreciseLocation);
    injectCurrentScreenAppState.set(next.injectCurrentScreenApp);
    injectRecentAppUsageState.set(next.injectRecentAppUsage);
    injectScreenTextState.set(next.injectScreenText);
    injectNotificationsState.set(next.injectNotifications);
    injectMemoryState.set(next.injectMemory);
    allowRepeatedMemorySearchState.set(next.allowRepeatedMemorySearch);
    memoryLimitState.set(next.memoryLimit);
    memoryLimitInputState.set(String(next.memoryLimit));
    injectionTimeoutSecondsState.set(next.injectionTimeoutSeconds);
    injectionTimeoutInputState.set(String(next.injectionTimeoutSeconds));
  };

  /// Applies only the fields changed by the current interaction to the local DSL state.
  const syncSettingsPatch = (
    patch: Partial<ExtraInfoInjectionSettings>
  ): void => {
    if (patch.masterEnabled !== undefined) {
      masterEnabledState.set(Boolean(patch.masterEnabled));
    }
    if (patch.persistInjectedContent !== undefined) {
      persistInjectedContentState.set(Boolean(patch.persistInjectedContent));
    }
    if (patch.injectTime !== undefined) {
      injectTimeState.set(Boolean(patch.injectTime));
    }
    if (patch.injectBattery !== undefined) {
      injectBatteryState.set(Boolean(patch.injectBattery));
    }
    if (patch.injectWeather !== undefined) {
      injectWeatherState.set(Boolean(patch.injectWeather));
    }
    if (patch.injectLocation !== undefined) {
      injectLocationState.set(Boolean(patch.injectLocation));
    }
    if (patch.usePreciseLocation !== undefined) {
      usePreciseLocationState.set(Boolean(patch.usePreciseLocation));
    }
    if (patch.injectCurrentScreenApp !== undefined) {
      injectCurrentScreenAppState.set(Boolean(patch.injectCurrentScreenApp));
    }
    if (patch.injectRecentAppUsage !== undefined) {
      injectRecentAppUsageState.set(Boolean(patch.injectRecentAppUsage));
    }
    if (patch.injectScreenText !== undefined) {
      injectScreenTextState.set(Boolean(patch.injectScreenText));
    }
    if (patch.injectNotifications !== undefined) {
      injectNotificationsState.set(Boolean(patch.injectNotifications));
    }
    if (patch.injectMemory !== undefined) {
      injectMemoryState.set(Boolean(patch.injectMemory));
    }
    if (patch.allowRepeatedMemorySearch !== undefined) {
      allowRepeatedMemorySearchState.set(
        Boolean(patch.allowRepeatedMemorySearch)
      );
    }
    if (patch.memoryLimit !== undefined) {
      const memoryLimit = Math.floor(Number(patch.memoryLimit));
      memoryLimitState.set(memoryLimit);
      memoryLimitInputState.set(String(memoryLimit));
    }
    if (patch.injectionTimeoutSeconds !== undefined) {
      const timeoutSeconds = Number(patch.injectionTimeoutSeconds);
      injectionTimeoutSecondsState.set(timeoutSeconds);
      injectionTimeoutInputState.set(String(timeoutSeconds));
    }
  };

  /// Updates the Compose state immediately and persists the same patch asynchronously.
  const persistSettings = (
    patch: Partial<ExtraInfoInjectionSettings>,
    successMessage = ""
  ): Promise<void> => {
    syncSettingsPatch(patch);
    errorMessageState.set("");
    successMessageState.set(successMessage);
    return saveSettingsInMain(patch)
      .then(() => undefined)
      .catch(error => {
        const message = error instanceof Error ? error.message : String(error || "unknown");
        successMessageState.set("");
        errorMessageState.set(`${text.saveErrorPrefix}${message}`);
      });
  };

  const applyMemorySettings = (): Promise<void> | void => {
    const limit = Number(memoryLimitInputState.value.trim());

    if (!Number.isFinite(limit) || limit < 1) {
      successMessageState.set("");
      errorMessageState.set(`${text.saveErrorPrefix}${text.invalidMemoryLimitMessage}`);
      return;
    }

    return persistSettings({
      memoryLimit: Math.floor(limit),
    });
  };

  const applyInjectionTimeoutSettings = (): Promise<void> | void => {
    const timeoutSeconds = Number(injectionTimeoutInputState.value.trim());

    if (
      !Number.isFinite(timeoutSeconds) ||
      timeoutSeconds < MIN_INJECTION_TIMEOUT_SECONDS ||
      timeoutSeconds > MAX_INJECTION_TIMEOUT_SECONDS ||
      !Number.isInteger(timeoutSeconds)
    ) {
      successMessageState.set("");
      errorMessageState.set(`${text.saveErrorPrefix}${text.invalidInjectionTimeoutMessage}`);
      return;
    }

    return persistSettings({ injectionTimeoutSeconds: timeoutSeconds });
  };

  const summaryLines = [
    masterEnabledState.value ? text.summaryMasterEnabled : text.summaryMasterDisabled,
    persistInjectedContentState.value
      ? text.summaryPersistEnabled
      : text.summaryPersistDisabled,
    injectTimeState.value ? text.summaryTimeEnabled : text.summaryTimeDisabled,
    injectBatteryState.value ? text.summaryBatteryEnabled : text.summaryBatteryDisabled,
    injectWeatherState.value ? text.summaryWeatherEnabled : text.summaryWeatherDisabled,
    injectLocationState.value ? text.summaryLocationEnabled : text.summaryLocationDisabled,
    usePreciseLocationState.value
      ? text.summaryPreciseLocationEnabled
      : text.summaryPreciseLocationDisabled,
    injectCurrentScreenAppState.value
      ? text.summaryCurrentScreenAppEnabled
      : text.summaryCurrentScreenAppDisabled,
    injectRecentAppUsageState.value
      ? text.summaryRecentAppUsageEnabled
      : text.summaryRecentAppUsageDisabled,
    injectScreenTextState.value
      ? text.summaryScreenTextEnabled
      : text.summaryScreenTextDisabled,
    injectNotificationsState.value
      ? text.summaryNotificationsEnabled
      : text.summaryNotificationsDisabled,
    injectMemoryState.value
      ? `${text.summaryMemoryEnabled} (${text.memoryLimitLabel}: ${memoryLimitState.value}; ${
          allowRepeatedMemorySearchState.value
            ? text.summaryMemoryRepeatEnabled
            : text.summaryMemoryRepeatDisabled
        })`
      : text.summaryMemoryDisabled,
    text.summaryInjectionTimeout.replace(
      "{seconds}",
      injectionTimeoutSecondsState.value.toString()
    ),
    text.summaryRulesHint,
  ];

  const rootChildren: ComposeNode[] = [
    ctx.UI.Row({ verticalAlignment: "center" }, [
      ctx.UI.Icon({ name: "settings", tint: "primary", size: 24 }),
      ctx.UI.Spacer({ width: 8 }),
      ctx.UI.Text({
        text: text.toolboxTitle,
        style: "headlineSmall",
        fontWeight: "bold",
      }),
    ]),
    ctx.UI.Text({
      text: text.toolboxSubtitle,
      style: "bodyMedium",
      color: "onSurfaceVariant",
    }),
    ctx.UI.Surface(
      {
        fillMaxWidth: true,
        shape: { cornerRadius: 12 },
        containerColor: "secondaryContainer",
      },
      [
        ctx.UI.Row(
          { padding: { horizontal: 14, vertical: 12 }, verticalAlignment: "center" },
          [
            ctx.UI.Icon({ name: "info", tint: "onSecondaryContainer", size: 18 }),
            ctx.UI.Spacer({ width: 8 }),
            ctx.UI.Text({
              text: text.toolboxBanner,
              style: "bodySmall",
              color: "onSecondaryContainer",
            }),
          ]
        ),
      ]
    ),

    createSectionTitle(ctx, "settings", text.masterSectionTitle),
    createToggleCard(
      ctx,
      text.masterToggleTitle,
      text.masterToggleDescription,
      masterEnabledState.value,
      checked => {
        return persistSettings({ masterEnabled: checked });
      }
    ),
    createToggleCard(
      ctx,
      text.persistToggleTitle,
      text.persistToggleDescription,
      persistInjectedContentState.value,
      checked => {
        return persistSettings({ persistInjectedContent: checked });
      }
    ),
    createInjectionTimeoutConfigSection(
      ctx,
      text,
      injectionTimeoutInputState.value,
      value => {
        injectionTimeoutInputState.set(value);
      },
      applyInjectionTimeoutSettings
    ),

    createSectionTitle(ctx, "bolt", text.itemsSectionTitle),
    createInjectionItemsCard(
      ctx,
      [
        {
          title: text.timeToggleTitle,
          subtitle: text.timeToggleDescription,
          checked: injectTimeState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectTime: checked });
          },
        },
        {
          title: text.batteryToggleTitle,
          subtitle: text.batteryToggleDescription,
          checked: injectBatteryState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectBattery: checked });
          },
        },
        {
          title: text.weatherToggleTitle,
          subtitle: text.weatherToggleDescription,
          checked: injectWeatherState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectWeather: checked });
          },
        },
        {
          title: text.locationToggleTitle,
          subtitle: text.locationToggleDescription,
          checked: injectLocationState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectLocation: checked });
          },
        },
        {
          title: text.preciseLocationToggleTitle,
          subtitle: text.preciseLocationToggleDescription,
          checked: usePreciseLocationState.value,
          enabled: injectLocationState.value,
          onCheckedChange: checked => {
            return persistSettings({ usePreciseLocation: checked });
          },
        },
        {
          title: text.currentScreenAppToggleTitle,
          subtitle: text.currentScreenAppToggleDescription,
          checked: injectCurrentScreenAppState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectCurrentScreenApp: checked });
          },
        },
        {
          title: text.recentAppUsageToggleTitle,
          subtitle: text.recentAppUsageToggleDescription,
          checked: injectRecentAppUsageState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectRecentAppUsage: checked });
          },
        },
        {
          title: text.screenTextToggleTitle,
          subtitle: text.screenTextToggleDescription,
          checked: injectScreenTextState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectScreenText: checked });
          },
        },
        {
          title: text.notificationsToggleTitle,
          subtitle: text.notificationsToggleDescription,
          checked: injectNotificationsState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectNotifications: checked });
          },
        },
        {
          title: text.memoryToggleTitle,
          subtitle: text.memoryToggleDescription,
          checked: injectMemoryState.value,
          onCheckedChange: checked => {
            return persistSettings({ injectMemory: checked });
          },
        },
      ],
      createMemoryConfigSection(
        ctx,
        text,
        injectMemoryState.value,
        allowRepeatedMemorySearchState.value,
        checked => {
          return persistSettings({ allowRepeatedMemorySearch: checked });
        },
        memoryLimitInputState.value,
        value => {
          memoryLimitInputState.set(value);
        },
        applyMemorySettings
      )
    ),

    createSectionTitle(ctx, "checkCircle", text.summarySectionTitle),
    ctx.UI.Card(
      {
        fillMaxWidth: true,
        shape: { cornerRadius: 12 },
        containerColor: "primaryContainer",
        elevation: 1,
      },
      [
        ctx.UI.Column(
          { padding: 16, spacing: 8 },
          summaryLines.map((line, index) =>
            ctx.UI.Text({
              key: `summary-${index}`,
              text: line,
              style: index === summaryLines.length - 1 ? "bodySmall" : "bodyMedium",
              color: "onPrimaryContainer",
            })
          )
        ),
      ]
    ),
  ];

  if (successMessageState.value.trim()) {
    rootChildren.push(
      ctx.UI.Card({ containerColor: "primaryContainer", fillMaxWidth: true }, [
        ctx.UI.Row({ padding: { horizontal: 14, vertical: 12 }, verticalAlignment: "center" }, [
          ctx.UI.Icon({ name: "checkCircle", tint: "onPrimaryContainer" }),
          ctx.UI.Spacer({ width: 8 }),
          ctx.UI.Text({
            text: successMessageState.value,
            style: "bodyMedium",
            color: "onPrimaryContainer",
          }),
        ]),
      ])
    );
  }

  if (errorMessageState.value.trim()) {
    rootChildren.push(
      ctx.UI.Card({ containerColor: "errorContainer", fillMaxWidth: true }, [
        ctx.UI.Row({ padding: { horizontal: 14, vertical: 12 }, verticalAlignment: "center" }, [
          ctx.UI.Icon({ name: "error", tint: "onErrorContainer" }),
          ctx.UI.Spacer({ width: 8 }),
          ctx.UI.Text({
            text: errorMessageState.value,
            style: "bodyMedium",
            color: "onErrorContainer",
          }),
        ]),
      ])
    );
  }

  return ctx.UI.LazyColumn(
    {
      fillMaxSize: true,
      padding: 16,
      spacing: 16,
      onLoad: async () => {
        if (!hasInitializedState.value) {
          hasInitializedState.set(true);
          try {
            syncSettings(await readSettings());
            errorMessageState.set("");
          } catch (error) {
            const message = error instanceof Error ? error.message : String(error || "unknown");
            errorMessageState.set(`${text.saveErrorPrefix}${message}`);
          }
        }
      },
    },
    rootChildren
  );
}

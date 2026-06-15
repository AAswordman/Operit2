import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import {
  applySettingsPatch,
  createDefaultSettings,
  loadSettings,
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
  return withContext<ExtraInfoInjectionSettings>("main", {}, function() {
    return loadSettings();
  });
}

async function saveSettingsInMain(
  patch: Partial<ExtraInfoInjectionSettings>
): Promise<ExtraInfoInjectionSettings> {
  return withContext<ExtraInfoInjectionSettings>("main", { patch }, function() {
    return saveSettings(patch);
  });
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
  };

  const currentSettings = (): ExtraInfoInjectionSettings => ({
    masterEnabled: masterEnabledState.value,
    persistInjectedContent: persistInjectedContentState.value,
    injectTime: injectTimeState.value,
    injectBattery: injectBatteryState.value,
    injectWeather: injectWeatherState.value,
    injectLocation: injectLocationState.value,
    usePreciseLocation: usePreciseLocationState.value,
    injectCurrentScreenApp: injectCurrentScreenAppState.value,
    injectRecentAppUsage: injectRecentAppUsageState.value,
    injectScreenText: injectScreenTextState.value,
    injectNotifications: injectNotificationsState.value,
    injectMemory: injectMemoryState.value,
    allowRepeatedMemorySearch: allowRepeatedMemorySearchState.value,
    memoryLimit: memoryLimitState.value,
  });

  const persistSettings = async (
    patch: Partial<ExtraInfoInjectionSettings>,
    successMessage = ""
  ): Promise<void> => {
    const next = applySettingsPatch(currentSettings(), patch);
    syncSettings(next);
    errorMessageState.set("");
    successMessageState.set(successMessage);

    try {
      syncSettings(await saveSettingsInMain(patch));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error || "unknown");
      successMessageState.set("");
      errorMessageState.set(`${text.saveErrorPrefix}${message}`);
    }
  };

  const applyMemorySettings = (): void => {
    const limit = Number(memoryLimitInputState.value.trim());

    if (!Number.isFinite(limit) || limit < 1) {
      successMessageState.set("");
      errorMessageState.set(`${text.saveErrorPrefix}${text.invalidMemoryLimitMessage}`);
      return;
    }

    persistSettings({
      memoryLimit: Math.floor(limit),
    });
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
        persistSettings({ masterEnabled: checked });
      }
    ),
    createToggleCard(
      ctx,
      text.persistToggleTitle,
      text.persistToggleDescription,
      persistInjectedContentState.value,
      checked => {
        persistSettings({ persistInjectedContent: checked });
      }
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
            persistSettings({ injectTime: checked });
          },
        },
        {
          title: text.batteryToggleTitle,
          subtitle: text.batteryToggleDescription,
          checked: injectBatteryState.value,
          onCheckedChange: checked => {
            persistSettings({ injectBattery: checked });
          },
        },
        {
          title: text.weatherToggleTitle,
          subtitle: text.weatherToggleDescription,
          checked: injectWeatherState.value,
          onCheckedChange: checked => {
            persistSettings({ injectWeather: checked });
          },
        },
        {
          title: text.locationToggleTitle,
          subtitle: text.locationToggleDescription,
          checked: injectLocationState.value,
          onCheckedChange: checked => {
            persistSettings({ injectLocation: checked });
          },
        },
        {
          title: text.preciseLocationToggleTitle,
          subtitle: text.preciseLocationToggleDescription,
          checked: usePreciseLocationState.value,
          enabled: injectLocationState.value,
          onCheckedChange: checked => {
            persistSettings({ usePreciseLocation: checked });
          },
        },
        {
          title: text.currentScreenAppToggleTitle,
          subtitle: text.currentScreenAppToggleDescription,
          checked: injectCurrentScreenAppState.value,
          onCheckedChange: checked => {
            persistSettings({ injectCurrentScreenApp: checked });
          },
        },
        {
          title: text.recentAppUsageToggleTitle,
          subtitle: text.recentAppUsageToggleDescription,
          checked: injectRecentAppUsageState.value,
          onCheckedChange: checked => {
            persistSettings({ injectRecentAppUsage: checked });
          },
        },
        {
          title: text.screenTextToggleTitle,
          subtitle: text.screenTextToggleDescription,
          checked: injectScreenTextState.value,
          onCheckedChange: checked => {
            persistSettings({ injectScreenText: checked });
          },
        },
        {
          title: text.notificationsToggleTitle,
          subtitle: text.notificationsToggleDescription,
          checked: injectNotificationsState.value,
          onCheckedChange: checked => {
            persistSettings({ injectNotifications: checked });
          },
        },
        {
          title: text.memoryToggleTitle,
          subtitle: text.memoryToggleDescription,
          checked: injectMemoryState.value,
          onCheckedChange: checked => {
            persistSettings({ injectMemory: checked });
          },
        },
      ],
      createMemoryConfigSection(
        ctx,
        text,
        injectMemoryState.value,
        allowRepeatedMemorySearchState.value,
        checked => {
          persistSettings({ allowRepeatedMemorySearch: checked });
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

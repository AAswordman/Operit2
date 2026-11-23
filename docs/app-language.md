# Application language

Open **Settings → Global Behavior Settings → App language**. The choices are
**Follow system**, **中文**, **English**, and **Français**, matching the app's
existing Chinese, English, and French translations. Native language names stay
recognizable when the surrounding interface uses a different language.

The initial choice is Follow system. Selection takes effect after saving, without
restarting or replacing the current navigation stack. Follow system uses Flutter's
locale resolution against `AppLocalizations.supportedLocales` and responds to
device language changes. An explicit choice stays active across system language
changes and application restarts.

`UserPreferencesManager` stores `app_language` (`system`, `zh`, `en`, or `fr`) in
`user_preferences.preferences.json`, using the existing synchronized personal
preference store. Each configured app engine loads the choice before showing its
main UI and observes committed changes. This setting is independent of theme
snapshots and saved theme configurations. Invalid persisted values are reported
as errors.

The main and detached chat windows, plugin floating windows, desktop widget
windows, application notification suffixes, and background work presentation
use the selected application language. Plugin UI receives the resolved language
through its existing localized session contracts. A plugin's own translations
determine which of its text is available in each language.

When adding a translated app language, update the ARB resources and `AppLanguage`
choices together. Regenerate localization sources with `fvm flutter gen-l10n`
from `apps/flutter/app`. The language tests verify that the selectable languages
match the generated supported locales.

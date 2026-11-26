# Extension page retention

The official extension list stays mounted while its route is in the shell's
back stack. Opening an extension detail and returning preserves the list's
State, ScrollPosition and accumulated candidates. Installing an extension in
the detail page refreshes the shared catalog; its original list row stays in
place and shows the installed action.

`OfficialExtensionsScreenRoute` uses `retainInBackStack`, like extension detail
routes. Popping the list itself releases the page. While another page covers
the list, `RetainedPage` suspends its rendering and `PageActivityMixin` detaches
its catalog subscription. Returning reattaches and refreshes current data.

A `stableScreenKey` establishes page identity but does not retain mounted State.
`AppContent` only renders inactive cached pages when their retention policy
allows it. Without `retainInBackStack`, the official list was removed from the
widget tree after the detail entrance transition, even though its widget was
still in `_screenCache`. Returning reused the widget configuration but created
new State and a new scroll position at the top.

The widget regression uses the production route policies and list/detail pages
with an in-memory Core connection. It scrolls a long official list, opens a
detail, returns, installs from the detail and returns again. It verifies page
and scroll-position identity, the original offset and row, inactive tickers,
offline catalog access, and disposal after popping the list. Coverage includes
phone and tablet widths with navigation animation enabled and disabled.

Run from `apps/flutter/app`:

```sh
fvm flutter test test/extension_catalog_manager_test.dart
```

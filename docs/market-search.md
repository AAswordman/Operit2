# Marketplace search input

The extension manager shares one search field between installed extensions and
the marketplace. Searches use committed text; an active, nonempty IME composing
range is held until the input method commits it.

The screen listens to `TextEditingController` so composition-only updates reach
the search query. `TextField.onChanged` only reports text changes, so using it
while ignoring composing text lost commits that retained the same characters.
Both an empty range and a valid collapsed range represent completed composition.
Selection-only changes do not rebuild search results when the text is unchanged.
The keyboard search action completes editing through Flutter's standard behavior;
the controller listener observes the resulting commit as well.

Marketplace matching loads every listing page and filters the committed query.
Regression coverage in `extension_catalog_manager_test.dart` exercises both IME
commit forms, a matching plugin on page two, ordinary typing, no matches, the
keyboard search action, and closing search.

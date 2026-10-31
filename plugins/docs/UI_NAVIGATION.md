# Plugin page navigation

Use the existing Compose context method to open another page:

```ts
await ctx.navigate('toolpkg:com.example.plugin:ui:details', { file: 'src/main.ts' });
```

The destination's registered navigation surface determines where the host presents it. A route registered with `surface: 'workspace'` opens in the side workspace. Other routes continue through the application router.

```ts
ToolPkg.registerUiRoute({
  id: 'details',
  route: 'toolpkg:com.example.plugin:ui:details',
  runtime: 'compose_dsl',
  screen: detailsScreen,
});
ToolPkg.registerNavigationEntry({
  id: 'details',
  route: 'toolpkg:com.example.plugin:ui:details',
  surface: 'workspace',
  title: { en: 'Details', zh: '详情' },
});
```

The host validates the enabled workspace contribution against the source plugin. Repeated navigation to the same workspace route focuses the existing tab and updates its `input`; arguments remain separate from host context such as `workspacePath` and `chatId`. The plugin reads the arguments with `ctx.useState('input', {})`.

No placement or instance identifier is needed at the call site. Omit the second argument when the destination needs no data:

```ts
await ctx.navigate('toolpkg:com.example.plugin:ui:details');
```

Navigation uses the existing Compose session response stream. It does not depend on an event handler's return value and can also run in asynchronous callbacks registered by a composition-owned `useEffect`. The page must still be mounted; disposing the composition releases its listeners.

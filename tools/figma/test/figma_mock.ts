/** API-shaped test host, checked against official typings in production code. */
export function createFigmaMock() {
  let next = 0;
  const nodes = new Map<string, any>(),
    variables = new Map<string, any>(),
    collections = new Map<string, any>(),
    images = new Map<string, Uint8Array>();
  const messages: any[] = [];
  const page: any = {
    type: "PAGE",
    id: "page",
    children: [],
    selection: [],
    findAll(predicate: (node: any) => boolean) {
      const result: any[] = [];
      const visit = (node: any) => {
        for (const child of node.children ?? []) {
          if (predicate(child)) result.push(child);
          visit(child);
        }
      };
      visit(this);
      return result;
    },
    appendChild(n: any) {
      n.parent?.children.splice(n.parent.children.indexOf(n), 1);
      n.parent = this;
      this.children.push(n);
    },
  };
  function create(type: string): any {
    const data = new Map<string, string>();
    const n: any = {
      type,
      id: `1:${++next}`,
      name: type,
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      visible: true,
      opacity: 1,
      cornerRadius: 0,
      rotation: 0,
      fills: [],
      strokes: [],
      strokeWeight: 1,
      clipsContent: false,
      paddingLeft: 0,
      paddingRight: 0,
      boundVariables: {},
      resize(w: number, h: number) {
        this.width = w;
        this.height = h;
      },
      setPluginData(k: string, v: string) {
        data.set(k, v);
      },
      getPluginData(k: string) {
        return data.get(k) || "";
      },
      setBoundVariable(k: string, v: any) {
        if (v) this.boundVariables[k] = { type: "VARIABLE_ALIAS", id: v.id };
        else delete this.boundVariables[k];
      },
      remove() {
        if (this.parent) {
          this.parent.children.splice(this.parent.children.indexOf(this), 1);
          this.parent = null;
        }
        nodes.delete(this.id);
      },
      async exportAsync() {
        return '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><path d="M0 0h20v20z"/></svg>';
      },
    };
    if (["FRAME", "COMPONENT"].includes(type)) {
      n.children = [];
      n.appendChild = page.appendChild;
      n.findAll = (predicate: any) => {
        const result: any[] = [];
        const walk = (x: any) => {
          for (const child of x.children ?? []) {
            if (predicate(child)) result.push(child);
            walk(child);
          }
        };
        walk(n);
        return result;
      };
    }
    if (type === "TEXT")
      Object.assign(n, {
        characters: "",
        textAutoResize: "WIDTH_AND_HEIGHT",
        fontName: { family: "Inter", style: "Regular" },
        fontSize: 14,
        textAlignHorizontal: "LEFT",
        lineHeight: { unit: "AUTO" },
        setRangeFontName() {},
        setRangeFontSize() {},
        setRangeFills() {},
        getStyledTextSegments(this: any): any[] {
          return this.characters
            ? [
                {
                  start: 0,
                  end: this.characters.length,
                  fontName: this.fontName,
                  fontSize: this.fontSize,
                  fills: this.fills,
                },
              ]
            : [];
        },
      });
    if (type === "TEXT") {
      let characters = "";
      Object.defineProperty(n, "characters", {
        enumerable: true,
        get: () => characters,
        set(value: string) {
          characters = value;
          // Model the host's auto-sizing side effect, not typographic metrics.
          // A fixed runtime text box must survive assigning the initial text.
          if (n.textAutoResize === "WIDTH_AND_HEIGHT") {
            n.width = Math.max(1, value.length * n.fontSize * 0.5);
            n.height = n.fontSize * 1.2;
          }
        },
      });
    }
    nodes.set(n.id, n);
    page.appendChild(n);
    return n;
  }
  const api: any = {
    mixed: Symbol("mixed"),
    currentPage: page,
    showUI() {},
    viewport: { center: { x: 0, y: 0 }, scrollAndZoomIntoView() {} },
    ui: {
      postMessage(m: any) {
        messages.push(m);
      },
    },
    async listAvailableFontsAsync() {
      return [
        { fontName: { family: "Inter", style: "Regular" } },
        { fontName: { family: "Inter", style: "Bold" } },
        { fontName: { family: "Noto Sans SC", style: "Regular" } },
      ];
    },
    async loadFontAsync() {},
    createFrame: () => create("FRAME"),
    createComponent: () => create("COMPONENT"),
    createRectangle: () => create("RECTANGLE"),
    createEllipse: () => create("ELLIPSE"),
    createText: () => create("TEXT"),
    createNodeFromSvg: () => create("FRAME"),
    base64Decode: (s: string) => new Uint8Array(Buffer.from(s, "base64")),
    base64Encode: (b: Uint8Array) => Buffer.from(b).toString("base64"),
    createImage(bytes: Uint8Array) {
      const hash = Buffer.from(bytes).toString("base64");
      images.set(hash, bytes);
      return { hash };
    },
    getImageByHash(hash: string) {
      return { getBytesAsync: async () => images.get(hash) };
    },
    variables: {
      createVariableCollection(name: string) {
        const c: any = {
          id: `collection-${++next}`,
          name,
          defaultModeId: "default",
          remove() {
            collections.delete(c.id);
          },
        };
        collections.set(c.id, c);
        return c;
      },
      createVariable(name: string, collection: any, type: string) {
        const v: any = {
          id: `variable-${++next}`,
          name,
          variableCollectionId: collection.id,
          resolvedType: type,
          valuesByMode: {},
          setValueForMode(mode: string, value: any) {
            this.valuesByMode[mode] = value;
          },
        };
        variables.set(v.id, v);
        return v;
      },
      getVariableByIdAsync: async (id: string) => variables.get(id),
      getVariableCollectionByIdAsync: async (id: string) => collections.get(id),
      setBoundVariableForPaint(paint: any, field: string, v: any) {
        return {
          ...paint,
          boundVariables: { [field]: { type: "VARIABLE_ALIAS", id: v.id } },
        };
      },
    },
  };
  return { api, nodes, variables, collections, messages };
}

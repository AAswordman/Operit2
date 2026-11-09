# Whitepaper PDF build

Run from the repository root with the project virtual environment:

```powershell
.venv/Scripts/python.exe tools/docs/build_whitepaper.py
```

The editable manuscript is `docs/whitepaper/Operit2论文版.md`. The builder writes
`output/pdf/Operit2技术白皮书-论文版.pdf`, refreshes the Chinese repository PDF,
and renders every page under `tmp/pdfs/operit2-paper/` for visual review.

Dependencies: ReportLab, matplotlib, Pillow, pypdf, pypdfium2. Font embedding uses
Windows SimSun, SimHei, Times New Roman and matplotlib's STIX General.

## Typesetting specification

- A4, 25 mm margins.
- Chinese title: SimHei, 22 pt; body: SimSun, 12 pt, 20 pt leading.
- First-level headings: SimHei, 14 pt; second-level headings: SimHei, 12 pt.
- Abstracts, captions, tables and references: 10.5 pt.
- Latin text: Times New Roman.
- Table captions above; exactly three horizontal rules (1.25/0.75/1.25 pt),
  without shading or vertical rules. Keep figures and tables with their captions.

## Editorial provenance

Writing guidance consulted: K-Dense Scientific Agent Skills, scientific-writing.
Kassis T, Agarwal V, He Y, et al. *Scientific Agent Skills: A Library of Procedural
Knowledge for Research Agents*. 2026. https://doi.org/10.48550/arXiv.2609.00065.

AI assisted with public-source retrieval, text editing and PDF generation.
Editorial workflow and typesetting instructions belong in this build record,
rather than in the technical paper's argument or references.

Implementation descriptions derive from repository documentation and source
inspection. No performance measurements or complete runtime test executions
were performed as part of document generation.

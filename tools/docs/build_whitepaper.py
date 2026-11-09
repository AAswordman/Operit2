"""Build the Chinese research-paper edition and its local visual QA artifacts."""

from __future__ import annotations

import html
import io
import json
import re
import shutil
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import pypdfium2 as pdfium
from PIL import Image as PILImage, ImageDraw
from pypdf import PdfReader
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_JUSTIFY, TA_LEFT
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas
from reportlab.platypus import (
    BaseDocTemplate, Flowable, Frame, Image, KeepTogether, PageBreak,
    PageTemplate, Paragraph, Spacer, Table, TableStyle,
)

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "docs/whitepaper/Operit2论文版.md"
OUTPUT = ROOT / "output/pdf/Operit2技术白皮书-论文版.pdf"
QA = ROOT / "tmp/pdfs/operit2-paper"
WIDTH, HEIGHT = A4
MARGIN = 25 * 72 / 25.4
TEXT_WIDTH = WIDTH - MARGIN * 2
INK = colors.black
GRAY = colors.HexColor("#555555")
RULE = colors.HexColor("#777777")
PALE = colors.white
TABLE_AUDIT = []


def register_fonts():
    """Embed locally installed Chinese and Latin TrueType fonts."""
    font_dir = Path("C:/Windows/Fonts")
    for name, filename in [("Song", "simsun.ttc"), ("Hei", "simhei.ttf"),
                           ("Times", "times.ttf"), ("TimesBold", "timesbd.ttf")]:
        pdfmetrics.registerFont(TTFont(name, str(font_dir / filename), subfontIndex=0))
    pdfmetrics.registerFontFamily("Song", normal="Song", bold="Hei", italic="Song")
    pdfmetrics.registerFontFamily("Times", normal="Times", bold="TimesBold", italic="Times")
    pdfmetrics.registerFont(TTFont("Math", str(Path(matplotlib.get_data_path()) / "fonts/ttf/STIXGeneral.ttf")))


def inline(text):
    """Escape Markdown text and preserve clickable URLs with mixed typography."""
    pieces = re.split(r"(https?://[^\s]+)", text.replace("`", ""))
    result = []
    for piece in pieces:
        if piece.startswith("http"):
            url = piece.rstrip(".。；;")
            suffix = piece[len(url):]
            result.append(f'<link href="{html.escape(url, quote=True)}" color="#234D66">'
                          f'<font name="Times">{html.escape(url)}</font></link>{suffix}')
        else:
            escaped = html.escape(piece)
            result.append(re.sub(r"[A-Za-z0-9][A-Za-z0-9 /_:.,*()=+;@'’–-]*",
                                 lambda m: '<font name="Times">' + m.group() + '</font>', escaped))
    formatted = "".join(result)
    for character, digit in zip("₀₁₂₃", "0123"):
        formatted = formatted.replace(character, f'<font name="Times"><sub>{digit}</sub></font>')
    for character in "∖∘":
        formatted = formatted.replace(character, f'<font name="Math">{character}</font>')
    return formatted


def styles():
    """Define journal-style paragraph hierarchy with readable Chinese leading."""
    base = dict(fontName="Song", fontSize=12, leading=20, textColor=colors.black,
                wordWrap="CJK", allowWidows=0, allowOrphans=0)
    return {
        "body": ParagraphStyle("body", **base, alignment=TA_JUSTIFY, firstLineIndent=24, spaceAfter=4),
        "h2": ParagraphStyle("h2", fontName="Hei", fontSize=14, leading=22,
                             spaceBefore=12, spaceAfter=8, textColor=INK, keepWithNext=True),
        "h3": ParagraphStyle("h3", fontName="Hei", fontSize=12, leading=20,
                             spaceBefore=8, spaceAfter=5, textColor=INK, keepWithNext=True),
        "title": ParagraphStyle("title", fontName="Hei", fontSize=22, leading=31,
                                alignment=TA_CENTER, textColor=INK, spaceAfter=12, wordWrap="CJK"),
        "subtitle": ParagraphStyle("subtitle", fontName="Hei", fontSize=14, leading=22,
                                   alignment=TA_CENTER, spaceAfter=10),
        "english_title": ParagraphStyle("english_title", fontName="TimesBold", fontSize=12,
                                        leading=15, alignment=TA_CENTER, textColor=INK, spaceAfter=12),
        "author": ParagraphStyle("author", fontName="Song", fontSize=12, leading=20,
                                 alignment=TA_CENTER, spaceAfter=4),
        "version": ParagraphStyle("version", fontName="Song", fontSize=10.5, leading=15,
                                  alignment=TA_CENTER, textColor=GRAY, spaceAfter=10),
        "abstract": ParagraphStyle("abstract", fontName="Song", fontSize=10.5, leading=15,
                                   wordWrap="CJK", alignment=TA_JUSTIFY, spaceAfter=7),
        "english_abstract": ParagraphStyle("english_abstract", fontName="Times", fontSize=10.5,
                                           leading=13.5, alignment=TA_JUSTIFY, spaceAfter=7),
        "keyword": ParagraphStyle("keyword", fontName="Song", fontSize=10.5, leading=15,
                                  spaceAfter=8, wordWrap="CJK"),
        "caption": ParagraphStyle("caption", fontName="Song", fontSize=10.5, leading=15,
                                  alignment=TA_CENTER, textColor=INK, spaceBefore=5, spaceAfter=10,
                                  wordWrap="CJK"),
        "table_caption": ParagraphStyle("table_caption", fontName="Song", fontSize=10.5,
                                        leading=15, alignment=TA_CENTER, spaceBefore=8, spaceAfter=6,
                                        keepWithNext=True, wordWrap="CJK"),
        "table": ParagraphStyle("table", fontName="Song", fontSize=10.5, leading=15.5,
                                wordWrap="CJK", alignment=TA_LEFT),
        "table_head": ParagraphStyle("table_head", fontName="Hei", fontSize=10.5, leading=15.5,
                                     wordWrap="CJK", textColor=INK),
        "reference": ParagraphStyle("reference", fontName="Song", fontSize=10.5, leading=16,
                                    wordWrap="CJK", leftIndent=18, firstLineIndent=-18, spaceAfter=10),
    }


class NumberedCanvas(canvas.Canvas):
    """Add accurate page totals without rebuilding the manuscript."""

    def __init__(self, *args, **kwargs):
        """Initialize the canvas and deferred page states."""
        super().__init__(*args, **kwargs)
        self.states = []

    def showPage(self):
        """Store each complete page before starting the next one."""
        self.states.append(dict(self.__dict__))
        self._startPage()

    def save(self):
        """Replay page states with final page numbering."""
        total = len(self.states)
        for state in self.states:
            self.__dict__.update(state)
            self.setStrokeColor(RULE)
            self.setLineWidth(0.35)
            self.line(MARGIN, 39, WIDTH - MARGIN, 39)
            self.setFillColor(GRAY)
            self.setFont("Song", 9)
            self.drawString(MARGIN, 26, "Operit开发团队")
            self.setFont("Times", 9)
            self.drawRightString(WIDTH - MARGIN, 26, f"{self._pageNumber} / {total}")
            super().showPage()
        super().save()


def header(c, doc):
    """Draw a restrained running title on every page."""
    c.saveState()
    c.setFont("Song", 9)
    c.setFillColor(GRAY)
    c.drawString(MARGIN, HEIGHT - 30, "Operit2 技术白皮书")
    c.drawRightString(WIDTH - MARGIN, HEIGHT - 30, "架构、路由机制与工程实践")
    c.setStrokeColor(RULE)
    c.setLineWidth(0.45)
    c.line(MARGIN, HEIGHT - 37, WIDTH - MARGIN, HEIGHT - 37)
    c.restoreState()


class Diagram(Flowable):
    """Render original vector diagrams directly into the PDF."""

    def __init__(self, kind):
        """Set diagram dimensions for the available text width."""
        super().__init__()
        self.kind = kind
        self.width = TEXT_WIDTH
        self.height = {"architecture": 213, "migration": 144, "routing": 245,
                       "handoff": 270, "engineering": 215}[kind]

    def draw(self):
        """Draw a device space or a migration commutative square."""
        c = self.canv

        def label(x, y, text, size=9, font="Song", color=INK):
            """Place a centered label in a diagram."""
            if self.kind == "handoff":
                c.setFillColor(colors.white)
                label_width = pdfmetrics.stringWidth(text, font, size)
                c.rect(x - label_width / 2 - 2, y - 2, label_width + 4, size + 4, fill=1, stroke=0)
            c.setFillColor(color)
            c.setFont(font, size)
            c.drawCentredString(x, y, text)

        def arrow(x1, y1, x2, y2, dashed=False, double=False):
            """Draw an arrow with optional dashes and reverse arrowhead."""
            import math
            c.setStrokeColor(INK)
            c.setFillColor(INK)
            c.setLineWidth(0.85)
            c.setDash(4, 3) if dashed else c.setDash()
            c.line(x1, y1, x2, y2)
            c.setDash()
            angle = math.atan2(y2 - y1, x2 - x1)
            for x, y, a in [(x2, y2, angle)] + ([(x1, y1, angle + math.pi)] if double else []):
                path = c.beginPath()
                path.moveTo(x, y)
                for offset in [0.45, -0.45]:
                    path.lineTo(x - 5.5 * math.cos(a + offset), y - 5.5 * math.sin(a + offset))
                path.close()
                c.drawPath(path, fill=1, stroke=0)

        if self.kind == "architecture":
            c.setStrokeColor(RULE)
            c.setLineWidth(0.6)
            c.roundRect(1, 2, TEXT_WIDTH - 2, 207, 6, fill=0, stroke=1)
            label(TEXT_WIDTH / 2, 190, "Space：成员关系、可达路由与存储授权", 10)
            centers = [85, TEXT_WIDTH / 2, TEXT_WIDTH - 85]
            for center, title in zip(centers, ["移动设备", "桌面设备", "长期在线设备"]):
                x = center - 61
                c.setFillColor(PALE)
                c.setStrokeColor(RULE)
                c.roundRect(x, 84, 122, 85, 4, fill=1, stroke=1)
                label(center, 152, title, 9.5)
                label(center, 133, "CoreNode", 11, "TimesBold")
                label(center, 113, "Core + 本地 Host", 9)
                label(center, 96, "能力与系统权限独立", 8, color=GRAY)
            arrow(146, 140, centers[1] - 61, 140, double=True)
            arrow(centers[1] + 61, 140, centers[2] - 61, 140, double=True)
            arrow(85, 80, 85, 56)
            arrow(centers[1], 80, centers[1], 56)
            arrow(centers[2], 80, centers[2], 56)
            c.setStrokeColor(RULE)
            c.setFillColor(PALE)
            c.roundRect(24, 28, TEXT_WIDTH - 48, 28, 3, fill=1, stroke=1)
            label(TEXT_WIDTH / 2, 38, "持久复制：事务、接收凭据与 Blob", 9.5)
            label(TEXT_WIDTH / 2, 12, "实时路由：call / watch / push · Binding 选择后续执行节点", 8.4, color=GRAY)
        elif self.kind == "migration":
            positions = [(106, 112), (TEXT_WIDTH - 106, 112),
                         (106, 32), (TEXT_WIDTH - 106, 32)]
            texts = ["旧状态 A", "Apply(A, H)", "迁移状态 m(A)", "共同结果"]
            for (x, y), text in zip(positions, texts):
                c.setFillColor(PALE)
                c.setStrokeColor(RULE)
                c.roundRect(x - 80, y - 17, 160, 34, 3, fill=1, stroke=1)
                label(x, y - 3, text, 10)
            arrow(186, 112, TEXT_WIDTH - 186, 112)
            arrow(186, 32, TEXT_WIDTH - 186, 32)
            arrow(106, 94, 106, 51)
            arrow(TEXT_WIDTH - 106, 94, TEXT_WIDTH - 106, 51)
            label(TEXT_WIDTH / 2, 126, "应用 H", 8.5)
            label(TEXT_WIDTH / 2, 46, "应用转换后的历史", 8.3)
            label(61, 71, "迁移 m", 8.5)
            label(TEXT_WIDTH - 60, 71, "迁移 m", 8.5)
        elif self.kind == "routing":
            middle = TEXT_WIDTH / 2
            for y, text in [(223, "Flutter / CLI / SDK"), (176, "真实 Core 方法与生成的 route 元数据")]:
                c.setStrokeColor(INK)
                c.roundRect(middle - 162, y - 14, 324, 29, 2, fill=0)
                label(middle, y - 3, text, 10.5)
            arrow(middle, 208, middle, 192)
            centers = [70, middle, TEXT_WIDTH - 70]
            for x, title, detail in zip(centers, ["Local", "Target / SpaceRoute", "SpaceBinding"],
                                        ["本地业务", "显式节点／空间方法", "绑定 key 与 generation"]):
                c.roundRect(x - 66, 105, 132, 45, 2, fill=0)
                label(x, 133, title, 10, "TimesBold")
                label(x, 115, detail, 9.5)
                arrow(middle, 161, x, 151)
            c.roundRect(4, 50, 132, 32, 2, fill=0)
            label(70, 62, "本地 dispatch", 10.5)
            arrow(70, 104, 70, 84)
            c.roundRect(155, 50, TEXT_WIDTH - 159, 32, 2, fill=0)
            label((155 + TEXT_WIDTH - 4) / 2, 62, "CoreNodeRouter + Peer runtime", 10.5)
            arrow(middle, 104, middle, 84)
            arrow(TEXT_WIDTH - 70, 104, TEXT_WIDTH - 70, 84)
            label(TEXT_WIDTH / 2, 13, "可信链路 → 逐跳校验 → 目标本地服务 → 沿调用链返回", 10.5)
        elif self.kind == "handoff":
            positions = [55, TEXT_WIDTH / 2, TEXT_WIDTH - 55]
            for x, title in zip(positions, ["接入 Router", "当前执行节点 A", "目标节点 B"]):
                label(x, 251, title, 10.5)
                c.setDash(3, 3)
                c.setStrokeColor(RULE)
                c.line(x, 240, x, 23)
                c.setDash()
            label(positions[1], 218, "工具结束、结果落盘", 10.5)
            arrow(positions[1], 190, positions[0], 190)
            label((positions[1] + positions[0]) / 2, 199, "返回 transition", 10.5)
            arrow(positions[0], 145, positions[2], 145)
            label(TEXT_WIDTH / 2, 155, "比较写入 Binding，传送 transition 与 generation", 10.5)
            label(TEXT_WIDTH / 2, 111, "目标校验归属，等待 requiredClock 后开始下一轮", 10.5)
            arrow(positions[2], 76, positions[0], 76)
            label(TEXT_WIDTH / 2, 86, "continuation 的后续事件", 10.5)
            label(TEXT_WIDTH / 2, 16, "用户继续接收同一条逻辑响应流", 10.5)
        elif self.kind == "engineering":
            layers = ["用户入口：Flutter / CLI / Web Access",
                      "类型化 CoreProxy 与真实业务方法",
                      "应用运行时：生命周期、工具循环与服务装配",
                      "基础契约：存储 / 模型 / 工具 / 插件 / Link",
                      "Host API → 各平台文件、终端、网络与交互实现"]
            for i, text in enumerate(layers):
                y = 190 - i * 42
                c.setStrokeColor(INK)
                c.roundRect(8, y - 14, TEXT_WIDTH - 16, 29, 2, fill=0)
                label(TEXT_WIDTH / 2, y - 3, text, 10.5)
                if i < len(layers) - 1:
                    arrow(TEXT_WIDTH / 2, y - 15, TEXT_WIDTH / 2, y - 26)


def equation(latex, number):
    """Render a mathematical display at print resolution with a separate number."""
    plt.rcParams.update({"mathtext.fontset": "stix", "font.family": "STIXGeneral"})
    fig = plt.figure(figsize=(10, 0.6))
    display_latex = latex.replace(r"\land", r"\wedge").replace(r"\ne ", r"\neq ")
    fig.text(0, 0.3, f"${display_latex}$", fontsize=15, color="#172C3A")
    buf = io.BytesIO()
    fig.savefig(buf, format="png", dpi=320, bbox_inches="tight", pad_inches=0.04, transparent=True)
    plt.close(fig)
    buf.seek(0)
    png = PILImage.open(buf)
    w, h = png.size
    display_w = min(w * 72 / 320, TEXT_WIDTH - 49)
    display_h = h / w * display_w
    image = Image(buf, width=display_w, height=display_h)
    num = Paragraph(f"({number})", ParagraphStyle("eqnum", fontName="Times", fontSize=10, alignment=2))
    table = Table([[image, num]], colWidths=[TEXT_WIDTH - 30, 30])
    table.setStyle(TableStyle([("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                               ("ALIGN", (0, 0), (0, 0), "CENTER"),
                               ("TOPPADDING", (0, 0), (-1, -1), 5),
                               ("BOTTOMPADDING", (0, 0), (-1, -1), 11)]))
    return table


def make_table(lines, st):
    """Lay out compact three-line tables with repeated column headings."""
    rows = [[cell.strip() for cell in line.strip().strip("|").split("|")] for line in lines]
    rows = [row for row in rows if not all(re.fullmatch(r"[: -]+", cell) for cell in row)]
    count = len(rows[0])
    widths = [TEXT_WIDTH * p for p in ([0.22, 0.36, 0.42] if count == 3 else [0.25, 0.75])]
    content = [[Paragraph(inline(cell), st["table_head" if i == 0 else "table"])
                for cell in row] for i, row in enumerate(rows)]
    table = Table(content, colWidths=widths, repeatRows=1, hAlign="CENTER")
    table.setStyle(TableStyle([
        ("LINEABOVE", (0, 0), (-1, 0), 1.25, INK),
        ("LINEBELOW", (0, 0), (-1, 0), 0.75, INK),
        ("LINEBELOW", (0, -1), (-1, -1), 1.25, INK),
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("LEFTPADDING", (0, 0), (-1, -1), 7),
        ("RIGHTPADDING", (0, 0), (-1, -1), 7),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
    ]))
    TABLE_AUDIT.append({"rows": len(rows), "font_pt": 10.5, "horizontal_rules_pt": [1.25, 0.75, 1.25],
                        "vertical_rules": 0, "background": None, "caption_position": "above"})
    return table


def manuscript(st):
    """Convert the editable Markdown manuscript into explicitly composed pages."""
    blocks = SOURCE.read_text(encoding="utf-8").split("\n\n")
    story, equation_number, page, abstract_mode = [], 0, 1, ""
    consumed = set()
    for block_index, raw in enumerate(blocks):
        if block_index in consumed:
            continue
        block = raw.strip()
        if not block:
            continue
        if block == "<!-- page -->":
            story.append(PageBreak())
            page += 1
            continue
        if block.startswith("# "):
            title = block[2:].replace("跨端智能体系统", "<br/>跨端智能体系统")
            story.append(Paragraph(title, st["title"]))
        elif page == 1 and block == "架构、路由机制与工程实践":
            story.append(Paragraph(block, st["subtitle"]))
        elif page == 1 and block.startswith("Operit2:"):
            story.append(Paragraph(block, st["english_title"]))
        elif block.startswith("作者："):
            story.append(Paragraph(inline(block[3:]), st["author"]))
        elif block.startswith("版本："):
            story.append(Paragraph(block[3:], st["version"]))
        elif block == "## 摘要":
            abstract_mode = "cn"
            story.append(Paragraph("摘　要", st["h3"]))
        elif block == "## Abstract":
            abstract_mode = "en"
            story.append(Paragraph("Abstract", st["h3"]))
        elif block.startswith("关键词：") or block.startswith("Keywords:"):
            story.append(Paragraph(inline(block), st["keyword"]))
        elif block.startswith("## "):
            if block == "## 参考文献" or block.startswith("## 附录 "):
                story.append(PageBreak())
            story.append(Paragraph(block[3:], st["h2"]))
        elif block.startswith("### "):
            story.append(Paragraph(block[4:], st["h3"]))
        elif block.startswith("$$"):
            equation_number += 1
            story.append(equation(block.strip("$\n"), equation_number))
        elif block.startswith(":::figure "):
            caption = blocks[block_index + 1].strip()
            assert re.match(r"^图 \d", caption), "Each figure requires a following caption"
            story.append(KeepTogether([Diagram(block.split()[1]), Paragraph(inline(caption), st["caption"])]))
            consumed.add(block_index + 1)
        elif block.startswith("|"):
            story.append(KeepTogether([make_table(block.splitlines(), st)]))
        elif re.match(r"^表 \d", block):
            table_block = blocks[block_index + 1].strip()
            assert table_block.startswith("|"), "Each table caption must precede its table"
            story.append(KeepTogether([Paragraph(inline(block), st["table_caption"]),
                                      make_table(table_block.splitlines(), st)]))
            consumed.add(block_index + 1)
        elif re.match(r"^图 \d", block):
            story.append(Paragraph(inline(block), st["caption"]))
        elif re.match(r"^\[\d+\] ", block):
            story.append(Paragraph(inline(block), st["reference"]))
        elif page == 1:
            story.append(Paragraph(inline(block), st["english_abstract" if abstract_mode == "en" else "abstract"]))
        else:
            story.append(Paragraph(inline(block), st["body"]))
    return story


def render_qa():
    """Render every PDF page and record text and geometry checks for review."""
    pdf = pdfium.PdfDocument(str(OUTPUT))
    thumbs = []
    for index in range(len(pdf)):
        image = pdf[index].render(scale=1.45).to_pil().convert("RGB")
        image.save(QA / f"page-{index + 1:02}.png")
        image.thumbnail((298, 421))
        tile = PILImage.new("RGB", (318, 452), "#DCE1E5")
        tile.paste(image, ((318 - image.width) // 2, 8))
        ImageDraw.Draw(tile).text((146, 433), str(index + 1), fill="#172C3A")
        thumbs.append(tile)
    for offset in range(0, len(thumbs), 6):
        subset = thumbs[offset:offset + 6]
        contact = PILImage.new("RGB", (318 * 3, 452 * 2), "#DCE1E5")
        for index, tile in enumerate(subset):
            contact.paste(tile, ((index % 3) * 318, (index // 3) * 452))
        contact.save(QA / f"contact-{offset // 6 + 1}.png")
    reader = PdfReader(OUTPUT)
    report = {"pages": len(reader.pages), "bytes": OUTPUT.stat().st_size, "tables": TABLE_AUDIT,
              "typography": {"body_pt": 12, "body_leading_pt": 20, "h2_pt": 14,
                             "h3_pt": 12, "table_pt": 10.5, "margins_mm": 25},
              "page_text": [{"page": i + 1, "characters": len(p.extract_text()),
                             "start": p.extract_text()[50:180]} for i, p in enumerate(reader.pages)]}
    (QA / "verification.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))


def main():
    """Build the paper, refresh its repository copy, and prepare local previews."""
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    QA.mkdir(parents=True, exist_ok=True)
    register_fonts()
    document = BaseDocTemplate(str(OUTPUT), pagesize=A4, leftMargin=MARGIN, rightMargin=MARGIN,
                               topMargin=MARGIN, bottomMargin=MARGIN,
                               title="Operit2：面向个人设备空间的跨端智能体系统——架构、路由机制与工程实践",
                               author="Operit开发团队",
                               subject="系统架构、声明式路由、跨平台工程、使用场景与展望")
    frame = Frame(MARGIN, MARGIN, TEXT_WIDTH, HEIGHT - MARGIN * 2, leftPadding=0, rightPadding=0,
                  topPadding=0, bottomPadding=0)
    document.addPageTemplates(PageTemplate(id="paper", frames=[frame], onPage=header))
    document.build(manuscript(styles()), canvasmaker=NumberedCanvas)
    shutil.copyfile(OUTPUT, ROOT / "docs/Operit2技术白皮书.pdf")
    render_qa()


if __name__ == "__main__":
    main()

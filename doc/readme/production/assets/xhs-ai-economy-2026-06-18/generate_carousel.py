from __future__ import annotations

from pathlib import Path
from typing import Iterable

from PIL import Image, ImageDraw, ImageFont


W, H = 1080, 1440
OUT = Path(__file__).resolve().parent

FONT_CANDIDATES = [
    "/System/Library/Fonts/STHeiti Medium.ttc",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
]


def font(size: int) -> ImageFont.FreeTypeFont:
    for path in FONT_CANDIDATES:
        if Path(path).exists():
            return ImageFont.truetype(path, size=size)
    return ImageFont.load_default()


F_TITLE = font(82)
F_TITLE2 = font(64)
F_H1 = font(54)
F_H2 = font(40)
F_BODY = font(34)
F_SMALL = font(25)
F_TINY = font(21)
F_NUM = font(66)
F_BIG_NUM = font(88)
F_NUM_SMALL = font(48)

INK = "#18212B"
MUTED = "#596474"
SUBTLE = "#738092"
PAPER = "#F7F3EA"
PANEL = "#FFFDF8"
LINE = "#D8D2C4"
RED = "#C94C4C"
GREEN = "#34796A"
BLUE = "#315D8F"
GOLD = "#B9822B"
DARK = "#243447"


def draw_round(draw: ImageDraw.ImageDraw, xy, radius, fill, outline=None, width=1):
    draw.rounded_rectangle(xy, radius=radius, fill=fill, outline=outline, width=width)


def text_len(draw: ImageDraw.ImageDraw, text: str, fnt) -> float:
    return draw.textlength(text, font=fnt)


def wrap(draw: ImageDraw.ImageDraw, text: str, fnt, max_w: int) -> list[str]:
    lines: list[str] = []
    current = ""
    for ch in text:
        trial = current + ch
        if text_len(draw, trial, fnt) <= max_w or not current:
            current = trial
        else:
            lines.append(current)
            current = ch
    if current:
        lines.append(current)
    return lines


def draw_wrapped(draw, text, xy, fnt, max_w, fill=INK, line_gap=12):
    x, y = xy
    for line in wrap(draw, text, fnt, max_w):
        draw.text((x, y), line, font=fnt, fill=fill)
        y += fnt.size + line_gap
    return y


def card_base(page: str, title: str, eyebrow: str = "AI革命、经济繁荣与分配失衡") -> tuple[Image.Image, ImageDraw.ImageDraw]:
    img = Image.new("RGB", (W, H), PAPER)
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, W, 18), fill=DARK)
    draw.text((72, 48), eyebrow, font=F_SMALL, fill=MUTED)
    draw.text((930, 48), page, font=F_SMALL, fill=MUTED)
    draw_wrapped(draw, title, (72, 116), F_H1, 900, fill=INK, line_gap=16)
    draw.line((72, 250, 1008, 250), fill=LINE, width=2)
    draw.text((72, 1360), "注：仅为个人视角分享，不构成确定预测或投资建议", font=F_TINY, fill=SUBTLE)
    return img, draw


def pill(draw, xy, text, fill, color="#FFFFFF", fnt=F_SMALL):
    x, y = xy
    pad_x, pad_y = 18, 10
    tw = text_len(draw, text, fnt)
    draw_round(draw, (x, y, x + tw + pad_x * 2, y + fnt.size + pad_y * 2), 18, fill)
    draw.text((x + pad_x, y + pad_y - 2), text, font=fnt, fill=color)


def metric_box(draw, xy, w, h, num, label, accent, note=None):
    x, y = xy
    draw_round(draw, (x, y, x + w, y + h), 28, PANEL, LINE, 2)
    draw.rectangle((x, y, x + 12, y + h), fill=accent)
    draw.text((x + 34, y + 30), num, font=F_NUM, fill=accent)
    yy = y + 118
    yy = draw_wrapped(draw, label, (x + 34, yy), F_BODY, w - 70, fill=INK, line_gap=9)
    if note:
        draw_wrapped(draw, note, (x + 34, yy + 12), F_SMALL, w - 70, fill=MUTED, line_gap=7)


def table_row(draw, y, cols: Iterable[tuple[str, int, str]], fill=None):
    x = 82
    if fill:
        draw_round(draw, (72, y - 12, 1008, y + 82), 18, fill)
    for text, width, color in cols:
        draw_wrapped(draw, text, (x, y), F_SMALL, width, fill=color, line_gap=5)
        x += width + 24


def flow_node(draw, xy, text, fill, outline=None, fnt=F_SMALL, color=INK):
    x, y, w, h = xy
    draw_round(draw, (x, y, x + w, y + h), 24, fill, outline or LINE, 2)
    lines = wrap(draw, text, fnt, w - 36)
    total = len(lines) * fnt.size + (len(lines) - 1) * 8
    yy = y + (h - total) / 2 - 2
    for line in lines:
        tw = text_len(draw, line, fnt)
        draw.text((x + (w - tw) / 2, yy), line, font=fnt, fill=color)
        yy += fnt.size + 8


def page1():
    img = Image.new("RGB", (W, H), "#F5F0E6")
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, W, H), fill="#F5F0E6")
    draw.ellipse((-180, 890, 360, 1430), fill="#DDE7DF")
    draw.ellipse((760, -160, 1230, 320), fill="#E8D8C2")
    draw.rectangle((0, 0, W, 22), fill=DARK)
    pill(draw, (72, 74), "AI经济影响视角分享", DARK)
    draw.text((72, 170), "AI会带来", font=F_TITLE, fill=INK)
    draw.text((72, 280), "经济繁荣，", font=F_TITLE, fill=BLUE)
    draw.text((72, 390), "还是萧条？", font=F_TITLE, fill=RED)
    draw_wrapped(draw, "到2035年，更可能不是两个极端，而是“先分化，后扩散”", (72, 545), F_H2, 870, fill=INK, line_gap=12)
    flow_node(draw, (355, 725, 370, 100), "AI能力突破", DARK, None, F_BODY, "#FFFFFF")
    draw.line((540, 825, 280, 935), fill=GREEN, width=8)
    draw.line((540, 825, 800, 935), fill=RED, width=6)
    flow_node(draw, (92, 955, 390, 150), "效率提升 / 新增长", "#EFF7F3", GREEN, F_SMALL)
    flow_node(draw, (598, 955, 390, 150), "分配失衡 / 弱增长", "#FAEEEE", RED, F_SMALL)
    draw_round(draw, (72, 1195, 1008, 1294), 24, PANEL, LINE, 2)
    draw_wrapped(draw, "我的判断：技术本身不是结局，分配机制决定走向。", (112, 1224), F_BODY, 840, INK, 10)
    draw.text((72, 1360), "AI革命、经济繁荣与分配失衡｜小红书轮播 01/08", font=F_TINY, fill=SUBTLE)
    return img


def page2():
    img, draw = card_base("02/08", "不是单边繁荣，也不是1929重演")
    draw_wrapped(draw, "我更倾向于一个审慎的中间路径：先分化、后扩散，并强烈依赖政策与制度设计。", (72, 292), F_BODY, 920, fill=MUTED, line_gap=13)
    draw_round(draw, (72, 455, 1008, 650), 32, "#FAEEEE", LINE, 2)
    draw.text((112, 500), "不是", font=F_H2, fill=RED)
    draw_wrapped(draw, "AI立刻带来全面繁荣的单边上行", (230, 500), F_BODY, 690, fill=INK, line_gap=12)
    draw_round(draw, (72, 720, 1008, 915), 32, "#FAEEEE", LINE, 2)
    draw.text((112, 765), "也不是", font=F_H2, fill=RED)
    draw_wrapped(draw, "机械复制1929式大萧条", (270, 765), F_BODY, 650, fill=INK, line_gap=12)
    draw_round(draw, (72, 990, 1008, 1225), 32, "#EEF3F8", BLUE, 3)
    draw.text((112, 1032), "更可能", font=F_H2, fill=BLUE)
    draw_wrapped(draw, "技术红利与分配摩擦并存：先集中，后看能否扩散。", (112, 1100), F_BODY, 830, fill=INK, line_gap=12)
    return img


def page3():
    img, draw = card_base("03/08", "采用很快，但宏观扩散仍不充分")
    draw_wrapped(draw, "AI已进入大规模扩散起点，但还远没有到“全社会生产函数被完全重写”的阶段。", (72, 292), F_BODY, 920, fill=MUTED, line_gap=13)
    metric_box(draw, (72, 450), 440, 260, "20.2%", "OECD企业2025年使用AI，较2023年的8.7%翻倍以上", BLUE)
    metric_box(draw, (568, 450), 440, 260, "17-20%", "美国企业已在业务中使用AI", GREEN)
    metric_box(draw, (72, 770), 440, 260, "55.03%", "欧盟大型企业2025年使用AI", GOLD)
    metric_box(draw, (568, 770), 440, 260, "6.02亿", "中国生成式AI用户规模", RED)
    draw_round(draw, (72, 1116, 1008, 1252), 28, "#EEF3F8", None, 0)
    draw_wrapped(draw, "解读：采用速度很快，但大企业、头部行业和更成熟组织仍然明显领先。", (112, 1154), F_BODY, 840, fill=INK, line_gap=10)
    return img


def page4():
    img, draw = card_base("04/08", "生产率确实上来了，但主要仍是局部增益")
    draw_wrapped(draw, "我更关注“局部、岗位、企业”层面的证据，而不是把微观提效直接等同于宏观繁荣。", (72, 292), F_BODY, 920, fill=MUTED, line_gap=13)
    metric_box(draw, (72, 438), 936, 190, "+14% / +25%", "客服场景：员工生产率首月提高14%，三个月后约25%", GREEN)
    metric_box(draw, (72, 676), 440, 230, "25-40%", "写作、咨询、管理等任务实验中的效率提升区间", BLUE)
    metric_box(draw, (568, 676), 440, 230, "1.1-1.2%", "生成式AI对应的总体劳动生产率潜在提升", GOLD)
    draw_round(draw, (72, 980, 1008, 1216), 30, "#FFF7E8", LINE, 2)
    draw.text((112, 1025), "2035前我的判断", font=F_H2, fill=INK)
    draw_wrapped(draw, "额外贡献大概率不是零，但更可能是每年约0.2—0.6个百分点的中等量级。", (112, 1092), F_BODY, 840, fill=INK, line_gap=12)
    return img


def page5():
    img, draw = card_base("05/08", "不是总量失业潮，而是结构先变")
    draw_wrapped(draw, "AI的第一批冲击，更可能表现为少招新人、提高门槛、压缩入门层级，而不是立刻出现总量性失业潮。", (72, 292), F_BODY, 920, fill=MUTED, line_gap=13)
    metric_box(draw, (72, 455), 440, 240, "40%", "AI将影响全球近40%的就业", BLUE)
    metric_box(draw, (568, 455), 440, 240, "60%", "先进经济体约60%的就业会受影响", RED)
    draw_round(draw, (72, 768, 1008, 1118), 30, PANEL, LINE, 2)
    draw.text((112, 810), "早期分化信号", font=F_H2, fill=INK)
    table_row(draw, 895, [("高自动化潜力职业", 300, MUTED), ("岗位发布", 180, MUTED), ("-17%", 140, RED)], "#F8F5EE")
    table_row(draw, 988, [("高增强潜力职业", 300, MUTED), ("岗位发布", 180, MUTED), ("+22%", 140, GREEN)])
    table_row(draw, 1080, [("22—25岁高暴露职业", 300, MUTED), ("就业相对变化", 180, MUTED), ("-16%", 140, RED)], "#F8F5EE")
    draw_round(draw, (72, 1194, 1008, 1292), 24, "#EEF3F8", None, 0)
    draw_wrapped(draw, "风险重点：分配风险可能先于总量问题显性化。", (112, 1224), F_BODY, 850, fill=INK, line_gap=10)
    return img


def scenario_box(draw, xy, title, prob, global_gdp, china_gdp, accent, fill):
    x, y = xy
    draw_round(draw, (x, y, x + 290, y + 470), 28, fill, LINE, 2)
    draw.text((x + 28, y + 28), title, font=F_H2, fill=accent)
    draw.text((x + 28, y + 98), prob, font=F_NUM_SMALL, fill=accent)
    draw_wrapped(draw, "概率估计", (x + 28, y + 160), F_SMALL, 230, MUTED, 6)
    draw.line((x + 28, y + 210, x + 262, y + 210), fill=LINE, width=2)
    draw.text((x + 28, y + 238), "全球年均GDP", font=F_TINY, fill=MUTED)
    draw.text((x + 28, y + 274), global_gdp, font=F_SMALL, fill=INK)
    draw.text((x + 28, y + 348), "中国年均GDP", font=F_TINY, fill=MUTED)
    draw.text((x + 28, y + 384), china_gdp, font=F_SMALL, fill=INK)


def page6():
    img, draw = card_base("06/08", "中性情景概率最高：45%—55%")
    draw_wrapped(draw, "我把2026—2035年理解为三种情景。概率不是机械预测，而是基于当前证据的区间判断。", (72, 292), F_BODY, 920, fill=MUTED, line_gap=13)
    scenario_box(draw, (72, 455), "乐观", "25-35%", "3.2-3.8%", "4.5-5.2%", GREEN, "#EFF7F3")
    scenario_box(draw, (395, 455), "中性", "45-55%", "2.6-3.2%", "3.8-4.6%", BLUE, "#EEF3F8")
    scenario_box(draw, (718, 455), "悲观", "15-25%", "1.8-2.5%", "2.8-3.8%", RED, "#FAEEEE")
    draw_round(draw, (72, 1020, 1008, 1238), 30, PANEL, LINE, 2)
    draw.text((112, 1062), "关键分岔点", font=F_H2, fill=INK)
    draw_wrapped(draw, "技术扩散是否足够广，培训与再分配是否跟上，竞争格局是否避免过度集中。", (112, 1130), F_BODY, 840, fill=INK, line_gap=12)
    return img


def page7():
    img, draw = card_base("07/08", "真正分水岭：红利能否扩散")
    draw_wrapped(draw, "AI政策不只是“多投一点科技”。真正决定宏观走向的是创新、分配、竞争与教育的组合。", (72, 292), F_BODY, 920, fill=MUTED, line_gap=13)
    items = [
        ("创新政策", "推动技术前沿与行业应用", BLUE),
        ("分配政策", "让效率红利传导到收入与消费", GREEN),
        ("竞争政策", "防止算力、云、模型生态过度锁定", GOLD),
        ("教育政策", "提升多数劳动者的AI协作能力", RED),
    ]
    coords = [(72, 470), (568, 470), (72, 790), (568, 790)]
    for (head, body, accent), (x, y) in zip(items, coords):
        draw_round(draw, (x, y, x + 440, y + 250), 30, PANEL, LINE, 2)
        draw.rectangle((x, y, x + 440, y + 16), fill=accent)
        draw.text((x + 34, y + 48), head, font=F_H2, fill=accent)
        draw_wrapped(draw, body, (x + 34, y + 120), F_BODY, 360, fill=INK, line_gap=10)
    draw_round(draw, (72, 1130, 1008, 1260), 28, "#243447", None, 0)
    draw_wrapped(draw, "如果只做技术投资，不做扩散与分配，AI更可能强化集中。", (112, 1168), F_BODY, 840, "#FFFFFF", line_gap=10)
    return img


def page8():
    img, draw = card_base("08/08", "问题不只是AI会不会更强，而是收益由谁拿走")
    draw_wrapped(draw, "如果政策与组织适配做对，AI大概率是增长利好；如果做错，AI可能先成为分配冲击，再通过需求和金融渠道拖累增长。", (72, 292), F_BODY, 920, fill=MUTED, line_gap=13)
    checks = [
        ("培训与转岗", "劳动者能否完成技能转换"),
        ("收入传导", "工资、社保与公共服务能否托住需求"),
        ("竞争政策", "算力、云、数据和模型红利是否过度集中"),
    ]
    y = 495
    for i, (head, body) in enumerate(checks, 1):
        accent = [BLUE, GREEN, GOLD][i - 1]
        draw_round(draw, (72, y, 1008, y + 190), 30, PANEL, LINE, 2)
        draw.ellipse((112, y + 48, 182, y + 118), fill=accent)
        draw.text((136, y + 62), str(i), font=F_H2, fill="#FFFFFF")
        draw.text((215, y + 42), head, font=F_H2, fill=INK)
        draw_wrapped(draw, body, (215, y + 108), F_BODY, 710, fill=MUTED, line_gap=10)
        y += 230
    draw_round(draw, (72, 1210, 1008, 1306), 24, DARK, None, 0)
    draw.text((112, 1238), "#AI时代 #经济观察 #收入分配 #职场趋势", font=F_SMALL, fill="#FFFFFF")
    return img


def main() -> None:
    pages = [page1(), page2(), page3(), page4(), page5(), page6(), page7(), page8()]
    for idx, img in enumerate(pages, 1):
        img.save(OUT / f"xhs-ai-economy-carousel-{idx:02d}.png", quality=95)


if __name__ == "__main__":
    main()

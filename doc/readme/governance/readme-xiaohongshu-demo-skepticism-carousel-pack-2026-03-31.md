# 小红书第八篇轮播版素材包：做AI游戏以后，我越来越不信demo了（2026-03-31）

审计轮次: 1

## Meta
- Owner: `liveops_community`
- Review Owner: `producer_system_designer`
- Channel: `小红书`
- Audience: `人类开发者 / 对 AI 游戏、agent 协作、项目判断和 demo 祛魅感兴趣的人`
- Positioning: `把第八篇长文版压成 4 页更适合小红书 feed 的轮播，用“先停住 -> demo vs project -> checklist -> 站队”的节奏，把“我越来越不信 demo / 更能看到本质了”这件事讲得更短、更直、更适合评论区接话`
- Review Status: `ready_for_publish`

## 使用说明
- 这是第八篇的轮播版，不替代长文版；它解决的是“手机端更容易读完”和“每页只记住一个判断点”。
- 标题、边界、互动问题必须和长文版保持一致：
  - 标题不变。
  - 不滑向泛赛道唱衰或“AI 都是泡沫”。
  - 评论区仍然收束到“你更吃第一眼很猛，还是更在意它能不能持续玩下去”。
- 轮播页的节奏要求：
  - 第 1 页先用强立场把人停住。
  - 第 2 页把 `demo` 和 `project` 的差别打清楚。
  - 第 3 页给出“我现在会先问什么”，把判断落地。
  - 第 4 页负责站队和评论区提问。

## 发布结构
- 建议页数：`4`
- 形式：`1 页封面 + 2 页判断卡 + 1 页收束页`
- 推荐标题：
```text
做AI游戏以后，我越来越不信demo了
```

## 逐页文案

### 第 1 页 / 封面
```text
做AI游戏以后

我越来越
不信demo了

不是不信AI
是会先想
它能撑多久
```

### 第 2 页 / 核心判断
```text
demo只负责高光

30秒里看起来很聪明
和
真放进项目里不露馅

根本不是一回事

demo挑最好的一分钟
项目会把最差状态也摊出来
```

### 第 3 页 / 判断清单
```text
我现在会先问

换个场景还灵不灵
多玩几次会不会露馅
出问题了能不能查
成本到底扛不扛得住
```

### 第 4 页 / 收束页
```text
现在我更在意的
不是第一眼很猛
是它能不能持续玩下去

你是哪种
更吃第一眼很猛
还是先看能不能持续玩下去
```

## 评论区引导
```text
你会更被“第一眼很猛”打动，还是会先想“这东西到底能不能真的放进项目里”？
```

## 配图与视觉说明
- HTML: `site/social/xiaohongshu-demo-skepticism-carousel.html`
- PNG:
  - `site/social/xiaohongshu-demo-skepticism-carousel-slide1.png`
  - `site/social/xiaohongshu-demo-skepticism-carousel-slide2.png`
  - `site/social/xiaohongshu-demo-skepticism-carousel-slide3.png`
  - `site/social/xiaohongshu-demo-skepticism-carousel-slide4.png`
- 使用方式：
  - 单页预览：`site/social/xiaohongshu-demo-skepticism-carousel.html?slide=1`
  - 逐页导出：`?slide=1` 到 `?slide=4`
  - 全部连看：直接打开不带参数的页面

## 视觉方向
- 沿用第八篇封面的 `项目审查板 / build review wall` 方向，但做成更适合小红书手机端滑读的 4 张卡。
- 每页都应该像同一个项目板里拆出来的不同区域，而不是四张互不相关的海报。
- 可用的视觉暗示：
  - 工业感编号、build 标签、review 章、分栏对照、粗粒度网格
  - 深色主卡 + 浅色检查卡 + 酸性荧光绿强调
  - checklist、warning stripe、投票格
- 不要出现：
  - 暖纸便签、胶带、手账贴纸
  - AI 科幻芯片海报
  - 已上线游戏实机宣传图

## 小红书笔记分析结论
- 综合评分：`9.0/10`
- 标题类型：`反直觉自述 + 经验判断`
- 风险等级：`🟢`
- 商业化程度：`7/10`
- 互动潜力判断：
  - 比单图封面版更适合让用户一路滑到“你更吃哪一种”这个站队点。
  - 第 2 页是整组轮播的核心页，负责把“第一眼很猛”和“真进项目”硬切开。
  - 第 3 页是最容易被收藏的一页，最好保留 checklist 感。
- 优先优化点：
  1. 第 2 页不要塞太多抽象词，保持直白对照。
  2. 第 3 页的 4 个问题要短，像真会拿来判断的清单。
  3. 第 4 页要留足空间给投票和评论区提问，不要再叠很多说明。

## 标签建议
```text
#AI
#AI游戏
#游戏开发
#独立游戏
#Agent
#开发日记
#产品思考
#demo
```

## 发布备注
- 如果发轮播版，caption 可以缩成 2-4 句，只负责承接，不要把整篇长文再贴一遍。
- 轮播版更适合接评论区站队，所以优先接“第一眼很猛 vs 能持续玩下去”，第 4 页可以更像原生投票卡，不要把讨论带回泛 AI 赛道。
- 如果评论区集中在“到底先问哪几个问题”，下一篇优先接 checklist 方向。
- 如果评论区更想看具体翻车例子，可以接“哪几种 demo 我现在一看就会先警惕”。

## 与长文版关系
- 长文版文档：`doc/readme/governance/readme-xiaohongshu-demo-skepticism-post-pack-2026-03-31.md`
- 轮播版不是重写观点，而是把长文版压成更适合小红书滑读的 4 个停顿点。
- 发布时二选一即可，不建议同一时间重复发长文版和轮播版。

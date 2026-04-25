import { createFileRoute } from "@tanstack/react-router";
import heroApiNetwork from "../assets/hero-api-network.png";
import logoKu0 from "../assets/logo-ku0.png";

const telegramUrl = "https://t.me/OpenHuge_ai";

const productSignals = [
  {
    body: "充值、购买、号池批发",
    label: "账户库",
  },
  {
    body: "质检、网关、同价接入",
    label: "中转库",
  },
  {
    body: "信息披露与交流",
    label: "信息库",
  },
  {
    body: "准入、质检、记录",
    label: "可信任",
  },
];

const analysisCards = [
  {
    body: "把 AI 账户充值、账号购买、订阅服务和号池批发统一归档，围绕交付、到账、售后和批量供应建立可追踪记录。",
    title: "账户库",
  },
  {
    body: "面向全网中转平台做可用性、模型覆盖、价格口径和计量一致性质检，并通过统一网关降低多平台接入成本。",
    title: "中转库",
  },
  {
    body: "持续整理最新 AI 产品、模型、平台、价格、风险和使用反馈，让信息披露和社区交流成为资源判断依据。",
    title: "信息库",
  },
  {
    body: "用准入规则、质检报告、交付记录、更新日志和公开交流补足资源交易中的信息差，而不是只展示低价和口头承诺。",
    title: "可信任机制",
  },
];

const categoryCards = [
  {
    body: "聚合可交付的 AI 账户资源，覆盖账户充值、账号购买、订阅代办和号池批发，强调交付确定性与售后边界。",
    points: ["账户充值与订阅续费", "账号购买与交付验收", "号池批发与批量供应"],
    title: "账户库",
  },
  {
    body: "对全网中转平台做标准化质检，筛选可接入资源，并通过统一网关按公开同价规则完成接入。",
    featured: true,
    points: [
      "全网中转平台质检",
      "统一网关、统一鉴权、统一计量",
      "同价接入与调用记录",
    ],
    title: "中转库",
  },
  {
    body: "沉淀最新 AI 相关信息，围绕产品更新、模型变化、价格调整、资源风险和使用反馈做披露与交流。",
    points: [
      "最新 AI 信息披露",
      "资源变更、风险和使用反馈",
      "频道交流与经验沉淀",
    ],
    title: "信息库",
  },
];

const relayFeatures = [
  {
    icon: "%",
    title: "开源质检探针",
    body: "提供协议一致性、模型身份、首包延迟、流式稳定、错误码和 token 计量探针，让用户在付款前先做基础鉴定。",
  },
  {
    icon: "ms",
    title: "自研巡检台",
    body: "对多个中转站、模型和 key 做定时巡检，持续记录可用率、时延分布、异常类型、价格口径和服务变更。",
  },
  {
    icon: "$",
    title: "模型身份鉴定",
    body: "通过固定样本、响应特征、能力边界和版本差异识别冒名、降级、替换模型，降低“买的是旗舰、用的是低配”的风险。",
  },
  {
    icon: "AI",
    title: "计量与成本核对",
    body: "核对倍率、token 统计、余额扣减、账单记录和缓存命中，让采购方知道真实成本，让服务商减少计费争议。",
  },
  {
    icon: "S",
    title: "商业质检报告",
    body: "输出可对外展示的供应商质检报告、接入建议、风险等级和复检时间，帮助用户采购，帮助优质平台建立信任。",
  },
  {
    icon: "!",
    title: "统一网关接入",
    body: "把通过质检的资源纳入统一网关，保留调用记录、异常归因和同价接入规则，让接入后仍然可追踪。",
  },
];

const accountCards = [
  {
    body: "覆盖充值路径、到账时间、凭证记录、失败处理和退款补偿规则，降低充值不确定性。",
    title: "账户充值",
  },
  {
    body: "明确账号类型、交付方式、登录验证、可用范围、换绑限制和售后边界。",
    title: "账号购买",
  },
  {
    body: "面向批量账号需求，记录库存、批量交付、可用率、异常补发、售后时限和长期供应能力。",
    title: "号池批发",
  },
  {
    body: "保留订单、充值、交付、验收、异常和补发记录，让账户资源可追踪、可复核。",
    title: "账户记录",
  },
];

const flowSteps = [
  {
    body: "收录账户充值、账号购买、号池批发、中转平台和 AI 信息线索，先完成分类、字段和边界整理。",
    label: "Step 01",
    title: "资源收录",
  },
  {
    body: "账户资源检查交付与售后，中转资源检查可用性、模型能力和计量口径，并把合格资源纳入统一网关。",
    label: "Step 02",
    title: "质检与接入",
  },
  {
    body: "发布资源状态、质检摘要、价格变化、风险提示和使用反馈，并在信息库中持续沉淀交流内容。",
    label: "Step 03",
    title: "披露与交流",
  },
];

const boundaryCards = [
  {
    body: "充值、购买和号池批发都需要明确交付方式、可用范围、异常处理和售后规则，不能只依赖口头承诺。",
    title: "账户资源必须说明交付边界",
  },
  {
    body: "中转平台状态会变化，统一网关会持续记录当前观察、历史趋势和检测时间，不给永久稳定承诺。",
    title: "中转质检不等于永久稳定",
  },
  {
    body: "AI 信息更新快，信息库会尽量标注来源、时间、适用范围和未验证项，避免把过期信息当成结论。",
    title: "信息披露需要标注来源和时间",
  },
  {
    body: "交流反馈会帮助判断资源状态，但平台会区分公开讨论、已核验事实和正式接入结论。",
    title: "公开交流不是无限背书",
  },
];

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      {
        title: "ku0.com - 库｜可信任 AI 资源库",
      },
      {
        content:
          "ku0.com 是可信任 AI 资源库，围绕账户库、中转库、信息库提供 AI 账户充值购买、号池批发、中转平台质检、统一网关同价接入和最新 AI 信息披露交流。",
        name: "description",
      },
    ],
  }),
  component: PublicHomeRoute,
});

function PublicHomeRoute() {
  return (
    <div className="ku-home" id="top">
      <Header />
      <main>
        <Hero />
        <ProductPositioning />
        <Categories />
        <RelayProducts />
        <AccountLibrary />
        <ServiceFlow />
        <InfoLibrary />
        <TrustBoundary />
      </main>
      <Footer />
    </div>
  );
}

function Header() {
  return (
    <header className="ku-site-header">
      <a aria-label="ku0.com 首页" className="ku-brand" href="#top">
        <span className="ku-brand-mark" aria-hidden="true">
          <img alt="" src={logoKu0} />
        </span>
        <span className="ku-brand-copy">
          <strong>ku0.com</strong>
          <small>可信任 AI 资源库</small>
        </span>
      </a>

      <nav aria-label="主导航" className="ku-nav-links">
        <a href="#analysis">产品定位</a>
        <a href="#categories">三大板块</a>
        <a href="#ledger">账户库</a>
        <a href="#relay">质检产品</a>
        <a href="#info">信息库</a>
        <a href="#flow">服务流程</a>
        <a href="#boundary">信任边界</a>
      </nav>

      <div className="ku-header-actions">
        <a className="ku-header-link" href="/login">
          登录
        </a>
        <a
          className="ku-header-action"
          href={telegramUrl}
          rel="noopener noreferrer"
          target="_blank"
        >
          频道
        </a>
      </div>
    </header>
  );
}

function Hero() {
  return (
    <section aria-labelledby="hero-title" className="ku-hero">
      <div className="ku-hero-media" aria-hidden="true">
        <img alt="" src={heroApiNetwork} />
      </div>
      <div className="ku-hero-shade" aria-hidden="true" />

      <div className="ku-hero-content">
        <p className="ku-eyebrow ku-hero-kicker">
          <span className="ku-status-dot" aria-hidden="true" />
          可信任 AI 资源库
        </p>
        <h1 id="hero-title">
          <span>ku0.com - 库</span>
          <span>可信任 AI</span>
          <span>资源库</span>
        </h1>
        <p className="ku-hero-lede">
          提供 AI 账户资源采购、Token 质检、统一网关接入与信息披露能力。
          用开源探针和自研巡检台识别模型冒用、掺水计量、延迟异常和服务波动。
          把充值、购买、号池批发等交付场景纳入可追踪记录，降低资源采购不确定性。
          持续发布质检结论、风险提示和行业动态，帮助用户做采购与接入决策。
        </p>

        <div aria-label="主要操作" className="ku-hero-actions">
          <a className="ku-button ku-button-primary" href="#categories">
            查看三大板块
          </a>
          <a
            className="ku-button ku-button-secondary"
            href={telegramUrl}
            rel="noopener noreferrer"
            target="_blank"
          >
            联系对接
          </a>
        </div>

        <div aria-label="核心产品服务" className="ku-trust-strip">
          {productSignals.map((signal) => (
            <div key={signal.label}>
              <strong>{signal.label}</strong>
              <span>{signal.body}</span>
            </div>
          ))}
        </div>
      </div>

      <aside aria-label="资源状态预览" className="ku-hero-console">
        <div className="ku-console-top">
          <span>ku0 资源工作台</span>
          <strong>三库能力建设中</strong>
        </div>
        <div className="ku-status-meter">
          <div>
            <span>Account</span>
            <strong>账户库</strong>
            <small>充值、购买、批发</small>
          </div>
          <div>
            <span>Relay</span>
            <strong>中转库</strong>
            <small>质检、网关、接入</small>
          </div>
          <div>
            <span>Info</span>
            <strong>信息库</strong>
            <small>披露、更新、交流</small>
          </div>
        </div>
        <div aria-label="履约链路" className="ku-verification-rail">
          {[
            ["01", "账户资源", "充值、购买、号池先归类"],
            ["02", "中转质检", "全网平台可用性复核"],
            ["03", "统一网关", "同价接入与调用记录"],
            ["04", "信息披露", "更新、风险、交流沉淀"],
          ].map(([step, title, body], index) => (
            <div
              className={`ku-verification-step${index === 0 ? " is-active" : ""}`}
              key={step}
            >
              <span>{step}</span>
              <strong>{title}</strong>
              <small>{body}</small>
            </div>
          ))}
        </div>
        <div className="ku-route-stack">
          {[
            ["中转库", "全网中转平台质检与统一网关同价接入", "接入", "teal"],
            ["账户库", "账户充值、购买、号池批发", "交易", "amber"],
            ["信息库", "最新 AI 信息披露与公开交流", "信息", "blue"],
          ].map(([title, body, status, color], index) => (
            <div
              className={`ku-route-item${index === 0 ? " is-active" : ""}`}
              key={title}
            >
              <span className={`ku-route-light ${color}`} />
              <div>
                <strong>{title}</strong>
                <small>{body}</small>
              </div>
              <b>{status}</b>
            </div>
          ))}
        </div>
      </aside>
    </section>
  );
}

function ProductPositioning() {
  return (
    <section
      aria-labelledby="analysis-title"
      className="ku-section ku-section-tight"
      id="analysis"
    >
      <SectionHeading
        body="ku0.com 的核心不是做泛信息导航，而是围绕 AI 账户资源、中转接入资源和行业信息资源建立可信任资源库，让用户更快完成获取、接入、判断和交流。"
        eyebrow="产品定位"
        title="把 AI 资源沉淀成可信任的三类库"
      />
      <div className="ku-signal-grid">
        {analysisCards.map((card, index) => (
          <article className="ku-signal-card" key={card.title}>
            <span className="ku-signal-index">
              {String(index + 1).padStart(2, "0")}
            </span>
            <h3>{card.title}</h3>
            <p>{card.body}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function Categories() {
  return (
    <section
      aria-labelledby="categories-title"
      className="ku-section ku-product-band"
      id="categories"
    >
      <SectionHeading
        align="left"
        body="三个板块分别解决“买账户资源”“接中转能力”“看最新信息”的问题，并通过准入、质检、记录和交流建立可信任的资源判断基础。"
        eyebrow="三大板块"
        title="账户库、中转库、信息库构成 ku0 的资源入口"
      />
      <div className="ku-category-grid">
        {categoryCards.map((card, index) => (
          <article
            className={`ku-category-card${card.featured ? " is-featured" : ""}`}
            key={card.title}
          >
            <span>{String(index + 1).padStart(2, "0")}</span>
            <h3>{card.title}</h3>
            <p>{card.body}</p>
            <ul>
              {card.points.map((point) => (
                <li key={point}>{point}</li>
              ))}
            </ul>
          </article>
        ))}
      </div>
    </section>
  );
}

function RelayProducts() {
  return (
    <section
      aria-labelledby="relay-title"
      className="ku-section ku-relay-section"
      id="relay"
    >
      <div className="ku-resource-layout">
        <SectionHeading
          align="left"
          body="ku0 质检产品面向购买账户、采购 token、接入 API 和筛选中转平台的真实决策场景，提供免费开源探针、批量巡检、模型身份鉴定、协议一致性检测、计量核对和商业级质检报告。"
          eyebrow="质检产品"
          title="开源探针与自研质检台，让 AI 中转先验货再采购"
        />
        <aside aria-label="质检口径" className="ku-merchant-panel">
          <span className="ku-merchant-label">产品矩阵</span>
          <strong>免费自测到商业报告</strong>
          <p>
            开源工具用于快速判断“能不能用、有没有掺水、是不是偷模型”；自研产品用于长期巡检、供应商评分、风险告警和统一网关接入前评估。
          </p>
          <a
            className="ku-button ku-button-secondary"
            href={telegramUrl}
            rel="noopener noreferrer"
            target="_blank"
          >
            申请接入
          </a>
        </aside>
      </div>

      <div className="ku-platform-layout">
        {relayFeatures.map((feature) => (
          <article className="ku-feature-row" key={feature.title}>
            <span className="ku-feature-icon" aria-hidden="true">
              {feature.icon}
            </span>
            <div>
              <h3>{feature.title}</h3>
              <p>{feature.body}</p>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

function AccountLibrary() {
  return (
    <section
      aria-labelledby="ledger-title"
      className="ku-section ku-ledger-section"
      id="ledger"
    >
      <SectionHeading
        align="left"
        body="账户库面向常见 AI 账户资源需求，整理账户充值、账号购买、订阅续费和号池批发能力，并围绕交付、到账、异常处理和批量供应建立清晰规则。"
        eyebrow="账户库"
        title="账户充值、购买、号池批发统一进入账户库"
      />
      <div className="ku-assurance-layout">
        <article className="ku-assurance-card is-primary">
          <span>账户策略</span>
          <h3>按资源类型区分交付与售后</h3>
          <p>
            充值类关注到账速度和凭证，购买类关注账号交付和可用性，号池批发关注批量稳定供应、库存状态和异常补发。
          </p>
        </article>
        <div className="ku-assurance-grid">
          {accountCards.map((card, index) => (
            <article key={card.title}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              <strong>{card.title}</strong>
              <p>{card.body}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function ServiceFlow() {
  return (
    <section
      aria-labelledby="flow-title"
      className="ku-section ku-flow-section"
      id="flow"
    >
      <SectionHeading
        body="账户库、中转库、信息库不是一次性页面，而是围绕收录、质检、发布、接入、披露和交流持续更新的资源体系。"
        eyebrow="服务流程"
        title="从资源收录到接入使用，持续沉淀可信记录"
      />
      <div className="ku-roadmap">
        {flowSteps.map((step, index) => (
          <div
            className={`ku-roadmap-item${index === 0 ? " is-current" : ""}`}
            key={step.label}
          >
            <span>{step.label}</span>
            <h3>{step.title}</h3>
            <p>{step.body}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

function InfoLibrary() {
  return (
    <section
      aria-labelledby="info-title"
      className="ku-section ku-education-section"
      id="info"
    >
      <div className="ku-education-copy">
        <p className="ku-eyebrow">信息库</p>
        <h2 id="info-title">最新 AI 相关信息披露与交流</h2>
        <p>
          信息库关注 AI
          产品、模型、平台、中转、价格和风险的最新变化，既做结构化披露，也保留来自真实使用场景的交流反馈。
        </p>
        <div className="ku-education-points">
          <span>最新 AI 产品与模型动态</span>
          <span>中转平台状态与风险披露</span>
          <span>频道交流与使用反馈沉淀</span>
        </div>
      </div>

      <aside aria-label="信息库入口" className="ku-education-panel">
        <span>Info Library</span>
        <strong>披露与交流</strong>
        <p>
          通过频道和页面同步最新信息、资源变更、质检更新和风险提示，让用户在交易或接入前看到更多上下文。
        </p>
        <a
          className="ku-button ku-button-primary"
          href={telegramUrl}
          rel="noopener noreferrer"
          target="_blank"
        >
          进入交流频道
        </a>
      </aside>
    </section>
  );
}

function TrustBoundary() {
  return (
    <section
      aria-labelledby="boundary-title"
      className="ku-section ku-boundary-section"
      id="boundary"
    >
      <div className="ku-principle-copy">
        <p className="ku-eyebrow">信任边界</p>
        <h2 id="boundary-title">可信资源库不是无限背书</h2>
        <p>
          平台提供公开字段、账户记录、中转质检、信息披露和风险提示；资源方仍需对来源、价格、交付、售后和合规承担责任。
        </p>
      </div>
      <div className="ku-principle-grid">
        {boundaryCards.map((card) => (
          <article key={card.title}>
            <h3>{card.title}</h3>
            <p>{card.body}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="ku-site-footer">
      <div>
        <strong>ku0.com</strong>
        <span>可信任 AI 资源库，围绕账户库、中转库、信息库持续更新。</span>
      </div>
      <div className="ku-footer-links">
        <a href={telegramUrl} rel="noopener noreferrer" target="_blank">
          Telegram 频道 @OpenHuge_ai
        </a>
        <a href="#top">回到顶部</a>
      </div>
    </footer>
  );
}

function SectionHeading({
  align = "center",
  body,
  eyebrow,
  title,
}: {
  align?: "center" | "left";
  body: string;
  eyebrow: string;
  title: string;
}) {
  return (
    <div
      className={`ku-section-heading${align === "left" ? " align-left" : ""}`}
    >
      <p className="ku-eyebrow">{eyebrow}</p>
      <h2>{title}</h2>
      <p>{body}</p>
    </div>
  );
}

# Business Requirements Document (BRD)

## 1. Document Control

| Field | Value |
|---|---|
| Product | AI Forex Trading Assistant |
| Document type | Business Requirements Document |
| Status | Draft — for stakeholder review |
| Prepared for | Kukuh TW (Creator and Product Owner) |
| Related documents | [Product overview](product-overview.md), [PRD](prd.md), [Architecture](architecture.md), [SaaS, administration, AI, and market data](saas-administration-ai-and-market-data.md), [Pricing, billing, and payment administration](pricing-billing-and-payment-administration.md), [Security](security.md), [Risk controls](risk-controls.md) |

This BRD reflects the intended commercial product built on top of the repository's current foundation. Where a requirement is already implemented, it is marked **Implemented**; everything else is a **Planned** business requirement that the engineering roadmap (see [PRD](prd.md)) must satisfy before commercial launch.

## 2. Purpose

This document defines *why* the AI Forex Trading Assistant exists as a business, *what* it must deliver to succeed commercially, and *what boundaries* it must never cross. It is the reference stakeholders use to approve scope, pricing, compliance posture, and go-to-market readiness. It does not specify implementation detail — that is the role of the [PRD](prd.md).

## 3. Business Background and Problem Statement

Retail forex traders who use MetaTrader 5 (MT5) typically analyze markets across multiple disconnected tools, then manually re-enter prices, lot sizes, and stop levels into their trading terminal. This creates three recurring business problems:

1. **Fragmented decision support.** Traders lack a single, structured place to turn raw price/indicator data into a consistent BUY/SELL/WAIT assessment with reasoning and risk levels.
2. **Unsafe automation trends.** Competing "AI trading bot" products often let AI output trigger broker execution directly, creating financial, security, and reputational risk for both users and operators.
3. **No trustworthy, auditable bridge between AI and execution.** Users who want AI-assisted analysis *and* to keep final authority over their own broker account have no product that guarantees human confirmation, independent risk validation, and a verifiable audit trail.

The business opportunity is a **Telegram-based SaaS** that monetizes structured AI analysis and a controlled confirmation/execution workflow around the user's own MT5 broker account — without ever taking custody of trading funds.

## 4. Business Objectives

| # | Objective | Rationale |
|---|---|---|
| BO-1 | Launch a subscription SaaS product that converts AI market analysis into recurring revenue | Fixed subscription pricing is predictable, auditable, and avoids conflict-of-interest models such as performance fees |
| BO-2 | Establish trust as a decision-support tool, not an autonomous trading bot | Differentiator against risk-laden "auto-trading AI" competitors; reduces regulatory and liability exposure |
| BO-3 | Preserve full user custody of trading funds at all times | The company must never become a money-transmitter, custodian, or broker-dealer |
| BO-4 | Support a safe demo-to-live user progression | Reduces support burden, disputes, and early-user financial harm, which protects brand and reduces churn |
| BO-5 | Provide operators with auditable, low-privilege administrative tooling | Required for support operations, incident response, and eventual compliance/regulatory review |
| BO-6 | Keep AI usage cost-recoverable and predictable | OpenAI usage is an operator cost; plan quotas must keep unit economics sustainable |
| BO-7 | Build toward a compliant, multi-jurisdiction-aware commercial launch | Forex-adjacent products face regulatory scrutiny (e.g., Bappebti in Indonesia, CFTC in the US, FCA in the UK) |

## 5. Scope

### 5.1 In scope

- A Telegram-delivered SaaS product for AI-assisted forex market analysis and human-confirmed MT5 trade execution.
- Fixed-tier subscription plans (Free Trial, Demo, Pro, Business) sold directly by the platform operator.
- Operator-owned OpenAI usage, metered and capped per plan.
- User-owned MT5 broker account connections (demo and live), never platform-custodied funds.
- Server-side (Rust) risk enforcement: ownership, permissions, live-trading gates, slippage, lot/position/trade limits, idempotency.
- A restricted, auditable operations/admin capability for platform staff.
- Audit logging of all material trading and account events.

### 5.2 Out of scope (current commercial phase)

- Holding, transmitting, or custodying user trading funds or deposits.
- Autonomous/unattended trade execution without per-order human confirmation.
- Performance-fee or profit-share pricing models.
- Brokerage services, market making, or acting as counterparty to any trade.
- Guaranteeing trading profit, win rate, or risk-free operation.
- Providing personalized financial, legal, or tax advice.
- Bring-your-own-key (BYOK) OpenAI support (deferred to a future enterprise phase, see [SaaS, administration, AI, and market data](saas-administration-ai-and-market-data.md)).

## 6. Stakeholders

| Stakeholder | Interest |
|---|---|
| Platform owner / operator (Kukuh TW) | Commercial success, brand trust, legal exposure, product direction |
| Subscribers (forex traders using MT5) | Useful, trustworthy analysis; full control of their funds and trades; transparent pricing |
| Operations / support staff | Tools to assist users and resolve billing/account issues without access to secrets or trading authority |
| Security / compliance function | Verifiable controls, auditability, incident response capability |
| Payment gateway partner (e.g., Midtrans, Xendit, Stripe) | Reliable, verifiable transaction and webhook integration |
| Brokers (external, unaffiliated) | Not a direct stakeholder, but the product's integrity depends on brokers remaining the sole custodian of funds and sole execution venue |
| Regulators (e.g., Bappebti, CFTC, FCA, depending on user jurisdiction) | Product must not misrepresent itself as a broker, fund custodian, or guaranteed-return investment vehicle |

## 7. Business Requirements

Each requirement has a stable ID for traceability into the PRD.

### 7.1 Trust and Fund Safety

| ID | Requirement | Status |
|---|---|---|
| BR-01 | The platform must never accept trading deposits or hold user funds; all trading capital remains at the user's broker | Implemented (architectural principle) |
| BR-02 | The platform must never process or facilitate broker withdrawals | Implemented (architectural principle) |
| BR-03 | Subscription payment and broker funding must be presented and processed as two entirely separate financial flows | Planned (billing not yet implemented) |
| BR-04 | Marketing and in-product copy must never claim guaranteed profit, risk-free trading, or "the AI cannot lose" | Implemented (documented product principle) |

### 7.2 AI-Assisted Analysis as a Paid Capability

| ID | Requirement | Status |
|---|---|---|
| BR-05 | The platform must provide structured AI market analysis (signal, bias, confidence, entry/SL/TP, reasoning, risk notes) as a core paid feature | Implemented (foundational endpoint) |
| BR-06 | AI output must always be treated as untrusted, decision-support data — never as an executable instruction | Implemented |
| BR-07 | AI usage must be metered per user/plan so operator OpenAI cost stays recoverable through subscription revenue | Planned |

### 7.3 Human-Controlled Execution

| ID | Requirement | Status |
|---|---|---|
| BR-08 | Every trade must require a distinct, explicit user confirmation step separate from trade configuration | Implemented |
| BR-09 | The platform must independently re-validate price, risk, and permissions immediately before execution, regardless of what the AI or user submitted | Implemented |
| BR-10 | Users must be able to verify every executed trade independently via the MT5 ticket | Implemented |
| BR-11 | Users must be able to halt new trading immediately via an emergency control, without losing access to safety-critical functions (view/close positions) | Implemented (kill switch) |

### 7.4 Risk Governance

| ID | Requirement | Status |
|---|---|---|
| BR-12 | The platform must enforce configurable risk limits (max lot size, open positions, trades/day, slippage) at the backend, not merely in the chat UI | Implemented |
| BR-13 | The platform must enforce daily-loss and per-trade-risk limits against authoritative broker equity/deal data before commercial launch | Planned |
| BR-14 | Live trading must require multiple independent, explicit gates (deployment mode, global flag, user opt-in, verified account, trading permission, per-order confirmation) so no single failure enables real-money trading by accident | Implemented |

### 7.5 Multi-Tenant SaaS and Data Isolation

| ID | Requirement | Status |
|---|---|---|
| BR-15 | Each user's accounts, analyses, trades, positions, risk settings, and audit history must be strictly isolated from every other user | Implemented |
| BR-16 | MT5 credentials must be encrypted at rest, with encryption keys stored outside the primary database | Implemented |
| BR-17 | No user-identifying trading secrets (passwords, tokens, keys) may ever be sent to OpenAI or appear in logs | Implemented |

### 7.6 Commercial / Subscription Model

| ID | Requirement | Status |
|---|---|---|
| BR-18 | The platform must offer tiered fixed-price subscriptions (Free Trial, Demo, Pro, Business) rather than per-trade or performance-based fees | Planned (pricing catalogue seeded; not sold yet) |
| BR-19 | Payment must be collected only through a hosted, PCI-compliant payment gateway; the platform must never store raw card/bank credentials | Planned |
| BR-20 | Payment confirmation must rely only on authoritative, signature-verified provider webhooks — never on browser redirects or user-submitted screenshots | Planned |
| BR-21 | Reaching a plan's usage limit must degrade gracefully (block new analysis/trades) and must never silently overcharge the user | Planned |
| BR-22 | Subscription expiry or cancellation must never auto-close an open broker position, and must always preserve safety-critical access (view/close positions, billing, security) | Planned |
| BR-23 | Paying for a higher-tier plan must never itself enable live trading; live eligibility remains gated separately | Planned (principle already documented) |

### 7.7 Administration and Operations

| ID | Requirement | Status |
|---|---|---|
| BR-24 | Operators must have a read-only administrative view of platform health, user/account counts, trading activity, and billing state | Implemented |
| BR-25 | The administrative surface must never expose plaintext credentials, API keys, or an unrestricted "trade as user" capability | Implemented |
| BR-26 | Production administrative access must require role-based access control and multi-factor authentication before commercial launch | Planned |
| BR-27 | Every sensitive administrative action (suspension, refund, credit, global flag change) must produce an immutable, attributable audit record | Partially implemented (audit logging exists; refund/credit workflows are planned) |

### 7.8 Compliance, Legal, and Regulatory

| ID | Requirement | Status |
|---|---|---|
| BR-28 | The product must clearly and repeatedly disclose that forex trading carries substantial risk of loss and that AI analysis can be wrong | Implemented (documentation and required in-product disclosures) |
| BR-29 | The product must direct users to verify broker legitimacy through the relevant official regulator (e.g., Bappebti, CFTC, FCA) rather than implying the platform vets brokers | Implemented (documented guidance) |
| BR-30 | Before public commercial launch, the operator must complete legal review of pricing, refund policy, privacy policy, and applicable financial-services regulation for each target jurisdiction | Planned |
| BR-31 | The platform must publish a refund/cancellation policy and retention policy prior to accepting payment | Planned |

### 7.9 Security

| ID | Requirement | Status |
|---|---|---|
| BR-32 | All internal service-to-service calls (Rust ↔ MT5 Bridge) must be authenticated and the bridge must never be exposed directly to the public internet | Implemented (dev default); production network hardening planned |
| BR-33 | The platform must maintain a documented threat model and update it as new capabilities (billing, Telegram UI) are added | Implemented, requires ongoing maintenance |
| BR-34 | The platform must complete security and dependency review, and ideally third-party penetration testing, before enabling live-money trading in production | Planned |

## 8. Assumptions

- Users already hold, or are willing to independently open, their own MT5-compatible broker account; the platform does not provide brokerage.
- The platform operator will own and pay for the OpenAI API project under the default SaaS model (no BYOK at launch).
- Indonesia is the initial primary market (proposed pricing in IDR, Bappebti guidance referenced), with expansion to other jurisdictions requiring separate legal review.
- Telegram remains the primary user interface for the initial commercial launch; a web app is not assumed in scope unless separately approved.
- Real MT5 terminal integration requires a Windows-based worker deployment; the current Linux/Docker development stack runs MT5 in mock mode only.

## 9. Constraints

- MetaTrader 5's official Python integration requires a Windows terminal session, which constrains production deployment topology (see [MT5 and the Rust–Python architecture](mt5-and-dual-tech-stack.md)).
- Proposed subscription prices are not final; they must be validated against OpenAI cost, infrastructure cost, payment-provider fees, and local tax rules before publication.
- Live trading cannot be offered until compliance review is complete for each jurisdiction in which it is enabled.
- The product must not become a payment intermediary for trading capital under any circumstance, which limits how billing and broker-funding flows can ever be combined.

## 10. Business Success Metrics

| Metric | Target framing |
|---|---|
| Paid conversion rate (Trial → Demo/Pro) | Primary funnel health indicator |
| Monthly recurring revenue (MRR) | Core commercial KPI once billing (Phase 1–2 of [pricing doc](pricing-billing-and-payment-administration.md)) ships |
| AI cost as a percentage of subscription revenue | Confirms unit economics are sustainable per plan |
| Percentage of trades reaching execution only after explicit confirmation | Should be 100%; any deviation is a critical defect |
| Support tickets per 1,000 active users related to billing confusion or fund-safety questions | Should trend down as disclosures and UX mature |
| Time from renewal failure to safe access restriction | Confirms grace-period and access-control logic is enforced correctly |
| Reconciliation queue size (payment/webhook mismatches) | Should stay near zero; growth signals a billing integration defect |

Success is explicitly **not** measured by trading profitability of users. It is measured by safe workflow execution, transparent billing, data isolation, and reliable policy enforcement — consistent with the [product overview's success criteria](product-overview.md#10-success-criteria).

## 11. Business Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Product is perceived or regulated as an unlicensed broker/investment adviser | Legal/regulatory action, forced shutdown | Strict fund-custody separation (BR-01/02), explicit non-advice disclaimers, jurisdiction-aware legal review before launch |
| A user mistakes AI output for a guaranteed signal and suffers a large loss, then disputes/publicizes it | Reputational and legal exposure | Mandatory risk disclosures, WAIT-biased AI instructions, human confirmation, documented "what the application does not do" |
| Payment fraud or webhook spoofing inflates entitlements without real payment | Revenue leakage, chargebacks | Signature-verified webhooks only, reconciliation queue, never trust client-reported payment status (see BR-20) |
| Administrative account compromise exposes user data or disables safety controls | Severe trust and security incident | RBAC + MFA for admin (BR-26), least-privilege roles, immutable audit trail (BR-27) |
| OpenAI cost scales faster than subscription revenue | Margin erosion | Per-plan hard quotas, no automatic overage billing at launch, usage metering (BR-07) |
| Live trading enabled prematurely for a user or market not yet compliance-reviewed | Regulatory and financial harm | Multi-gate live-trading model (BR-14), global feature flag defaults to disabled |
| Broker or MT5 outage during open positions leaves users unable to manage risk | User financial harm, support burden | Product guidance to always retain direct MT5/broker access; application must never be the sole access path |

## 12. Out of Scope for This Document

Detailed functional/non-functional requirements, API contracts, data models, and phased engineering delivery plans are defined in the [Product Requirements Document (PRD)](prd.md). Legal drafting of Terms of Service, Privacy Policy, and refund policy is a separate legal work product informed by, but not contained in, this BRD.

## 13. Glossary

| Term | Meaning |
|---|---|
| MT5 | MetaTrader 5, the trading terminal used to connect to a broker account |
| Trade intent | A proposed, non-executing trade configuration awaiting explicit user confirmation |
| Kill switch | User-controlled emergency stop that blocks new BUY/SELL orders while preserving safety access |
| BYOK | Bring Your Own (API) Key — a deferred model where a user supplies their own OpenAI key |
| Idempotency key | A client-generated token ensuring a confirmation cannot create a duplicate order |
| Live gate | One of several independent conditions that must all be true before a live-money order can execute |

## 14. Approval

This BRD requires sign-off from the platform owner before the corresponding PRD scope is committed to an engineering roadmap or a launch date is communicated externally.

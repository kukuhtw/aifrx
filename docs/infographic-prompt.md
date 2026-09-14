# Infographic Prompt: Explaining AI Forex Trading Assistant

## 1. Purpose

This is a ready-to-use prompt/brief for generating a one-page explainer infographic about this project — usable with an AI image generator, a design tool, or as a brief for a human designer. It covers five required questions, with the factually-correct content already extracted from this repository's own documentation so nothing has to be re-derived or guessed.

> A rendered version of this brief has already been built and published as a canvas artifact in this session — see the conversation for the link. This document is the reusable prompt behind it, kept so the infographic can be regenerated, adapted for another format (social card, print flyer, deck slide), or handed to a different tool later.

## 2. The Prompt

```text
Design a single-page infographic explaining an open-source-adjacent software
project called "AI Forex Trading Assistant." Audience: people encountering the
project for the first time on GitHub or social media — assume no prior
forex or trading-software knowledge. Tone: calm, precise, trustworthy —
NOT hype-y, no "get rich," no guaranteed-return language, no countdown
urgency. This is a financial-decision-support tool, so understatement is
more credible than excitement.

Cover exactly five sections, in this order:

1. WHAT IS THIS APPLICATION?
   A Telegram-based platform that turns live MT5 market data into a
   structured AI analysis (BUY / SELL / WAIT, with reasoning and risk
   notes), then requires the user to configure, review, and EXPLICITLY
   CONFIRM a trade before anything reaches their broker. State plainly:
   it is not an autonomous trading bot and cannot place a trade on its
   own. Core principle to feature prominently: "AI analyzes. The user
   decides. The backend controls the workflow. MT5 executes. The broker
   holds the funds." Optionally show a simple 4-step flow: analyze ->
   user reviews and confirms -> server re-checks risk and price ->
   broker executes and returns a verifiable ticket.

2. WHAT PROBLEMS DOES IT SOLVE?
   Present as 3-5 short problem -> solution pairs, for example:
   - Market analysis scattered across tools -> one structured signal
     with reasoning and risk levels
   - AI output could be mistaken for an executable instruction -> AI is
     analysis-only; every trade needs explicit human confirmation
   - Analysis and execution are disconnected, inviting manual-entry
     errors -> one traceable workflow from signal to confirmed order
   - Excessive automation removes user control -> an emergency stop
     blocks new trades instantly; every material action still requires
     confirmation
   Keep each pair to one short sentence. Do not list more than 5 -
   pick the most concrete ones, not an exhaustive catalogue.

3. HOW DOES IT RELATE TO MT5 AND THE BROKER?
   Show three distinct roles side by side, connected by simple arrows,
   with one line each:
   - The Broker: a licensed company that holds the user's funds, sets
     prices/spread/margin, and accepts or rejects every order.
   - MetaTrader 5 (MT5): trading software made by MetaQuotes - the
     connection ("the wire") between the user's device and the
     broker's own trade server. MT5 itself never holds funds.
   - This Application: adds AI analysis and a human-confirmed workflow
     on top of that connection. It never holds funds and forwards
     only what the user explicitly confirmed.
   Close with one summarizing line: "The broker owns the money and
   executes trades. MT5 is the wire. This app decides nothing on its
   own — it prepares, checks, and forwards only what the user
   confirmed."

4. WHY IS THE SOURCE CODE PUBLIC?
   Explain that for a tool that touches trading decisions and broker
   credentials, trust is better earned through code that can be
   independently verified than through claims alone — so anyone can
   check for themselves that AI output cannot directly trigger a
   trade, that credentials are encrypted and never sent to the AI
   provider, and that the risk controls described in the docs are
   actually implemented.
   MUST include, visually distinct (e.g. a bordered callout box), this
   exact caveat: the project is licensed under the PolyForm
   Noncommercial License 1.0.0. That means anyone may view, study,
   run, modify, and share the code freely for NONCOMMERCIAL purposes
   (personal learning, academic research, security review and
   auditing, nonprofit/government use, hobby projects) — but
   COMMERCIAL use is not covered and requires a separate written
   license from the copyright holder. Do not use the unqualified
   phrase "open source" without this caveat next to it; prefer
   "source-available, noncommercial license" or name the license
   directly.

5. CAN ANYONE USE THIS TO RUN AN AI FOREX TRADING BUSINESS?
   Answer in two distinct parts, not blended into one claim:
   (a) As code, today: no, not without separate permission - the
       PolyForm Noncommercial License only covers noncommercial use;
       operating a paid or monetized AI-forex service on this code is
       commercial use and requires contacting the project owner for a
       separate license.
   (b) Even with a commercial license: still not "clone and launch" -
       running a real AI-forex service requires financial-services/regulatory
       review appropriate to each jurisdiction, never holding user
       funds, production-grade multi-account isolation, and security
       and compliance sign-off before enabling any live (real-money)
       trading.

Required footer, in small type: a one-sentence risk disclosure
("Forex trading involves substantial risk and may result in loss of
capital. This application does not guarantee profit and is not
financial advice.") plus a credit line to the project and its GitHub
URL.

Visual direction: reuse the project's own existing visual language
rather than inventing a new one - a calm editorial fintech look:
off-white/paper background, deep forest-green (#0e6b50) as the primary
accent with a near-black forest green (#10251e) for a hero/header band,
a lime accent (#dff36b) used sparingly for emphasis, white cards with
soft shadows and rounded corners (~16px) for each section, a serif
display face (e.g. Georgia) for headings paired with a clean sans
(e.g. Inter/system-ui) for body text. No emoji, no stock-photo people,
no countdown/urgency devices, no gradients-for-their-own-sake. Icons
(if any) should be simple stroke-based line icons, not decorative
clip-art. Structure as clearly labeled numbered sections (01-05) so a
reader can scan it top to bottom in under a minute.
```

## 3. Notes for Whoever Uses This Prompt

- The license caveat in sections 4 and 5 is load-bearing, not optional styling — it reflects a deliberate decision by the project owner and should not be softened or dropped in any regenerated version.
- If regenerating for a different format (square social card, printed flyer, slide), keep all five numbered sections but it is fine to compress each to a single sentence; do not drop the license caveat even in a compressed version.
- Source facts for sections 1–3 are documented in more depth in [product-overview.md](product-overview.md) and [mt5-and-broker-relationship.md](mt5-and-broker-relationship.md); use those as the reference if a regeneration needs more supporting detail than the prompt above includes.
- Visual tokens (colors, fonts, radii) are lifted directly from the existing admin dashboard at `backend-rust/admin/index.html`, so a regenerated infographic stays visually consistent with the rest of the project rather than introducing a new, competing look.

## 4. Related Documentation

- [Product overview](product-overview.md) — problems solved, core principle, source for §1–2
- [MetaTrader 5, the broker, and this application](mt5-and-broker-relationship.md) — source for §3
- [README](../README.md) — project summary and full documentation index

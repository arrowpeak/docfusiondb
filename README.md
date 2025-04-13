# DocFusionDB

Welcome to **DocFusionDB**, an open-source project to build a high-performance document database using DataFusion and Postgres. We’re creating a system that leverages Postgres’ JSONB for document storage and DataFusion’s vectorized query engine for blazing-fast performance, targeting use cases like content management and real-time analytics.

We’re in the early stages of development, so this is the perfect time to get involved! 🎉

## 🌟 What is DocFusionDB?

DocFusionDB aims to:

- Store documents efficiently using Postgres’ JSONB.
- Query them at high speed with DataFusion’s Rust-based engine.
- Support hybrid workloads (documents + analytics) with a single SQL interface.

Think of it as a bridge between relational and NoSQL worlds, combining the best of both.

## 🚀 Getting Started

We’re still building the foundation, but you can set up a local environment to explore the prototype:

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Postgres](https://www.postgresql.org/download/) (v15 or later)
- [Docker](https://www.docker.com/get-started/) (optional, for easy setup)

### Setup

1. Clone the repo:

   ```bash
   git clone https://github.com/arrowpeak/docfusiondb.git
   cd docfusiondb
   ```

2. Start a Postgres instance (or use Docker):

   ```bash
   docker run -d --name docfusiondb-postgres -p 5432:5432 -e POSTGRES_PASSWORD=yourpassword postgres:15
   ```

3. Build the project:

   ```bash
   cargo build
   ```

4. Run the prototype (queries a sample JSONB table):

   ```bash
   cargo run --bin docfusiondb
   ```

## 🛠️ Contributing

We’re just starting out and would love your help! Here’s how you can contribute:

- Check out our issues for tasks (e.g., JSONB query improvements, docs).
- Submit ideas or bug reports via issues.
- Fork the repo, make changes, and open a pull request—we’ll review within 48 hours.

New to Rust or DataFusion? No worries! We’ll support you every step of the way.

## 📜 Roadmap

- **Phase 1 (0–6 months):** Build a basic JSONB query engine with DataFusion + Postgres.
- **Phase 2:** Optimize performance and add analytics features.
- **Phase 3:** Support advanced use cases like vector search and distributed setups.

See our full roadmap (`docs/roadmap.md`) for details (coming soon!).

## 📬 Get in Touch

- Join the conversation on Apache Arrow Slack in the `#docfusiondb` channel.
- Follow us on Twitter: [@DocFusionDB](https://twitter.com/DocFusionDB).
- Email: hello@docfusiondb.com (coming soon).

Let’s build the future of document databases together! 💡

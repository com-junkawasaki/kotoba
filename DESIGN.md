# 目的と結論（最小核のみ）

* **目的**: GP2系のグラフ書換えを核に、**ISO GQL**準拠クエリ、**MVCC+Merkle**永続、分散実行まで一貫させる。
* **最小構成（必須のみ）**

  1. **書換え仕様**: **DPO（Double Pushout）型付き属性グラフ**
  2. **クエリ**: **GQL → 論理プランIR（代数）**
  3. **更新**: **Patch-IR**（`addV/E, delV/E, setProp, relink`）
  4. **戦略**: **極小 Strategy-IR**（`once|exhaust|while|seq|choice|priority`）
  5. **実行**: **Rustコア**（プランナ/実行器/MVCC/ストア）
* **任意（外周）**: Jsonnet/XState/Serverless Workflow/JSONata は不要。必要時のみ外付け。

---

# 中核IR（JSON; 正規化→内容ハッシュを付与）

## 1) Rule-IR（DPO）

```json
{
  "rule": {
    "name": "tri_collapse",
    "types": {"nodes":["V"], "edges":["E"]},
    "L": {
      "nodes": [{"id":"u","type":"V"},{"id":"v","type":"V"},{"id":"w","type":"V"}],
      "edges": [
        {"id":"e1","src":"u","dst":"v","type":"E"},
        {"id":"e2","src":"v","dst":"w","type":"E"}
      ]
    },
    "K": {"nodes": [{"id":"u"},{"id":"w"}], "edges": []},
    "R": {
      "nodes": [{"id":"u"},{"id":"w"}],
      "edges": [{"id":"e3","src":"u","dst":"w","type":"E"}]
    },
    "NAC": [{"edges": [{"src":"u","dst":"w","type":"E"}]}],
    "guards": [
      {"ref":"deg_ge", "args":{"var":"u","k":2}},
      {"ref":"deg_ge", "args":{"var":"w","k":2}}
    ]
  }
}
```

* `guards.ref` は **名前付き述語**（Rustで実装; 索引に乗る述語のみ許可）。

## 2) Query-IR（GQL 論理プラン代数）

```json
{
  "plan": {
    "op": "Project", "cols": ["m"],
    "input": {
      "op": "Distinct",
      "input": {
        "op": "Expand", "edge": {"label":"FOLLOWS", "dir":"out"}, "toAs": "m",
        "from": {
          "op": "Filter", "pred": {"ge": [{"fn":"degree","args":["n"]}, 50]},
          "input": {"op":"NodeScan", "label":"Person", "as":"n"}
        }
      }
    },
    "limit": 100
  }
}
```

* 論理演算子: `NodeScan/IndexScan/Filter/Expand/Join/Project/Group/Sort/Limit/Distinct`。

## 3) Patch-IR（差分）

```json
{
  "patch": {
    "adds": {"v": [], "e": [{"src":"u","dst":"w","label":"E","props":{}}]},
    "dels": {"v": ["v"], "e": []},
    "updates": {"props": [], "relink": [{"from":"v","to":"u"}]}
  }
}
```

## 4) Strategy-IR（極小）

```json
{
  "strategy": {
    "op": "exhaust",           
    "rule": "sha256:…",        
    "order": "topdown",        
    "measure": "edge_count_nonincreasing"
  }
}
```

* サポート: `once | exhaust | while(pred) | seq(a,b) | choice(a,b,…) | priority(a>b)`
* `measure`/`pred` は **名前参照**（Rust側に実装; 浮動機能は不可）。

## 5) Catalog-IR（スキーマ/索引/不変量）

* ラベル/プロパティ型/索引/不変条件（例: 多重辺禁止・属性制約）。プランナと検証器の情報源。

---

# Rust 実行系（API骨格）

```rust
// GQL → 論理/物理プラン → 実行
fn parse_gql(src: &str) -> PlanIR;
fn plan_to_physical(ir: &PlanIR, cat: &Catalog) -> PhysPlan;
fn execute_plan(g: GraphRef, p: &PhysPlan) -> RowStream;

// DPO 書換え → Patch → MVCC
fn match_rule(g: GraphRef, r: &RuleIR, cat: &Catalog) -> Matches;
fn rewrite(g: GraphRef, r: &RuleIR, strat: &StrategyIR) -> Patch;
fn apply_patch(tx: &mut Tx, g: GraphRef, patch: Patch) -> GraphRef; // 純粋→Tx境界で副作用
fn commit(tx: &mut Tx, g: GraphRef, msg: &str) -> GraphRef;

// extern 述語/測度（索引利用可能なものに限定）
trait Externs { fn deg_ge(&self, v: Vid, k: u32) -> bool; fn edge_count_nonincreasing(&self, g0: GraphRef, g1: GraphRef) -> bool; }
```

---

# データモデル/永続化

* 物理: **列指向（AdjOut/AdjIn/Props）+ LSM（WAL→SST）+ 圧縮**。
* 一貫性: **MVCC**（列の世代版）/ 論理コミットは **Merkle DAG**。
* ID: **Stable ID ↔ Content Hash** の二層解決。

---

# 分散と最適化

* **パーティショニング**: 頂点ハッシュ分割 + 近傍キャッシュ/ミラー。
* **分散実行**: `Exchange` 演算子（分散Join/Expand）。
* **最適化**: 述語押下げ/結合順序DP/索引選択。**GQL/前件マッチを単一コストモデル**で最適化。

---

# LLM/ハッシュ/検証

* **JSON Schema** で IR を厳格検証 → 失敗はエラーを返し自己修復。
* **正規化**（キー順・UTF-8 NFC・数値表現固定）→ **内容ハッシュ**でID化（Unison流）。
* **静的チェック**: 近傍での臨界対探索/停止測度の健全性テスト。

---

# 運用チェックリスト

* `extern` 述語/測度の**ホワイトリスト**確定（索引に乗るもののみ）。
* Patch 適用の**不変量検査**（前/後条件）。
* LSM の**圧縮/TTL/レンジ分離**チューニング。
* ベンチ: **LDBC-SNB**（小規模→分散）+ 代表的リライト（例: 三角圧縮）。

---

# ロードマップ（最短）

1. **単ノード**: GQL→論理/物理→実行、Rule-IR→Patch、MVCC/Merkle。
2. **戦略器**: Strategy-IR 実装（`once/exhaust/priority`）+ 停止測度。
3. **分散**: パーティション/マルチRaft/`Exchange`/近傍キャッシュ。
4. **検証**: 臨界対テスト/不変量SMT/由来メタ（`(input, rule_hash, plan_hash) -> output`）。

---

# 付録（任意）: Node-Link 交換フォーマット

```json
{
  "nodes":[{"id":"u","labels":["Person"]},{"id":"v"},{"id":"w"}],
  "edges":[{"id":"e1","src":"u","dst":"v","label":"E"},{"id":"e2","src":"v","dst":"w","label":"E"}],
  "props":{"u":{"age":42}}
}
```

> 内部表現は**列指向+ID圧縮**、Node-Linkは**入出力/可視化**専用。

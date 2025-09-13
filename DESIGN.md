# 目的と結論（最小核のみ）

* **目的**: GP2系のグラフ書換えを核に、**ISO GQL**準拠クエリ、**MVCC+Merkle**永続、分散実行まで一貫させる。
* **最小構成（必須のみ）**

  1. **書換え仕様**: **DPO（Double Pushout）型付き属性グラフ**
  2. **クエリ**: **GQL → 論理プランIR（代数）**
  3. **更新**: **Patch-IR**（`addV/E, delV/E, setProp, relink`）
  4. **戦略**: **極小 Strategy-IR**（`once|exhaust|while|seq|choice|priority`）
  5. **実行**: **Rustコア**（プランナ/実行器/MVCC/ストア）

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


いいですね。\*\*「DPO + GQL(論理IR) + Patch + 極小Strategy + Rust」\*\*だけで、HTTPサーバを“グラフ駆動”で書く最小構成を示します。要点は、**ソケット等の副作用はRust**、**ルーティング～処理～レスポンス生成はグラフ書換え**で表します。

# 設計の骨格（最小で動く形）

* **データ型（Catalogの例）**
  `Route(method, pattern, handlerRef)` / `Middleware(order, fnRef)` /
  `Request(id, method, path, headers*, bodyRef)` / `Response(status, headers*, bodyRef)`
  物理ボディは **外部blob**（ファイル/LFS/オブジェクトストア）に置き、**グラフはメタ**（ハッシュ/長さ/MIME）だけ持つ。

* **イベントモデル**

  1. Rustがソケット受信→`Request`ノードを追加（Patch）
  2. **Strategy**を1回実行（`seq(route→mw→handler→finalize)`）
  3. 生成された`Response`ノードからRustが書き戻し

* **インデックス**
  `(:Route{method})`, `Route.pattern`（トライ/正規化セグメント）, `(:Middleware{order})` でヒットを絞る。
  前件照合は**GQLの論理プラン**で最適化（述語押下げ＋結合順序）。

---

# IR最小例（JSON）

## 1) ルーティング（DPO ルール）

`GET /ping` を `handlerRef="h_ping"` に振る最小例。

```json
{
  "rule": {
    "name": "route_match_ping",
    "L": {
      "nodes": [
        {"id":"req","type":"Request","props":{"method":"GET","path":"/ping"}},
        {"id":"r","type":"Route","props":{"method":"GET","pattern":"/ping"}}
      ],
      "edges": []
    },
    "K": {"nodes":[{"id":"req"},{"id":"r"}]},
    "R": {
      "nodes":[{"id":"req"},{"id":"r"}],
      "edges":[{"src":"req","dst":"r","type":"ROUTED"}]
    },
    "NAC": [{"edges":[{"src":"req","dst":"_","type":"ROUTED"}]}],
    "guards": []
  }
}
```

* `NAC`で「既にルーティング済みなら適用しない」を表現。
* 一般化するなら `pattern_match(req.path, r.pattern)` を **extern 述語**にする。

## 2) ミドルウェア適用（例：`X-Req-Id`付与）

```json
{
  "rule": {
    "name": "mw_request_id",
    "L": {
      "nodes":[
        {"id":"req","type":"Request"},
        {"id":"mw","type":"Middleware","props":{"order":100,"fnRef":"mw_reqid"}}
      ],
      "edges":[{"src":"req","dst":"mw","type":"NEXT_MW"}]
    },
    "K":{"nodes":[{"id":"req"},{"id":"mw"}],"edges":[{"src":"req","dst":"mw","type":"NEXT_MW"}]},
    "R":{
      "nodes":[{"id":"req"},{"id":"mw"}],
      "edges":[{"src":"req","dst":"mw","type":"APPLIED_MW"}]
    },
    "NAC":[{"edges":[{"src":"req","dst":"mw","type":"APPLIED_MW"}]}],
    "guards":[{"ref":"set_header_absent","args":{"node":"req","key":"x-req-id"}}]
  }
}
```

* `set_header_absent` は **extern**（索引に乗るヘッダ存在チェック＋生成をPatchへ反映）。

## 3) ハンドラ（`GET /ping` → 200 JSON）

```json
{
  "rule": {
    "name": "handler_ping",
    "L": {
      "nodes": [
        {"id":"req","type":"Request"},
        {"id":"r","type":"Route","props":{"pattern":"/ping","method":"GET"}}
      ],
      "edges": [{"src":"req","dst":"r","type":"ROUTED"}]
    },
    "K": {"nodes":[{"id":"req"},{"id":"r"}],"edges":[{"src":"req","dst":"r","type":"ROUTED"}]},
    "R": {
      "nodes":[
        {"id":"req"},{"id":"r"},
        {"id":"resp","type":"Response","props":{"status":200,"mime":"application/json","bodyRef":"blob:sha256:…"}}
      ],
      "edges":[
        {"src":"req","dst":"resp","type":"PRODUCES"}
      ]
    },
    "NAC":[{"edges":[{"src":"req","dst":"_","type":"PRODUCES"}]}],
    "guards":[]
  }
}
```

* `bodyRef` は外部blob（`{"ok":true}`をハッシュ保存）。
* 動的レスポンスなら `extern:"render_json"` で Patch 生成時に blob を作る。

## 4) Strategy（1リクエスト分の処理）

```json
{
  "strategy": {
    "op": "seq",
    "steps": [
      {"op":"once", "rule":"sha256:route_match_*", "order":"topdown"},
      {"op":"exhaust", "rule":"sha256:mw_*", "order":"topdown"},
      {"op":"once", "rule":"sha256:handler_*", "order":"topdown"},
      {"op":"once", "rule":"sha256:finalize_response", "order":"topdown"}
    ]
  }
}
```

* `finalize_response` はヘッダ確定・ログ連結などを行う最終化ルール。

---

# GQL（論理プラン）での前件マッチ例

ルーティング候補を引く論理プラン（イメージ）：

```json
{
  "plan": {
    "op":"Join",
    "how":"hash",
    "left": {"op":"Filter","pred":{"eq":[{"col":"label(n)"}, "Request"]},
             "input":{"op":"NodeScan","as":"n"}},
    "right":{"op":"Filter","pred":{"and":[
                {"eq":[{"prop":"m.method"}, {"prop":"n.method"}]},
                {"fn":"pattern_match","args":[{"prop":"n.path"},{"prop":"m.pattern"}]}
              ]},
             "input":{"op":"NodeScan","label":"Route","as":"m"}}
  }
}
```

この結果束縛を DPO 適用器へ流す（**クエリと書換えが同じ最適化器**を通る）。

---

# Rust 側の骨格（超要約）

```rust
struct Engine { catalog: Catalog, store: Store, planner: Planner, rewriter: Rewriter }

impl Engine {
    async fn handle(&self, raw: HttpRequest) -> HttpResponse {
        // 1) Requestノードを追加（Patch→MVCC）
        let (tx, g0) = self.store.begin();
        let req_ref = self.store.add_request(&tx, &raw).await; // bodyはblob保存、metaのみGraph
        let g1 = self.store.commit(tx, "new request");

        // 2) Strategy実行（seq: route→mw→handler→finalize）
        let strat = self.load_strategy("http_request_seq");
        let patch = self.rewriter.run(&g1, &strat).await?;
        let (tx2, _) = self.store.begin();
        let g2 = self.store.apply_patch(&tx2, g1, patch);
        let g3 = self.store.commit(tx2, "handled");

        // 3) Responseノードの取り出し
        let resp = self.store.lookup_response(g3, req_ref).await?;
        self.to_http(resp).await
    }
}
```

* **副作用**（ソケット入出力/Blob I/O）は**Tx境界外で**行い、**グラフ変更はPatchで純粋→Txで可視化**。
* ハンドラの“計算”が重い場合は **extern 関数**（Rust）に寄せ、IRは**構造と依存**だけを持つ。

---

# 実運用Tips（批判的に）

* **高スループット**：ルーティングはDPO前件をそのまま照合せず、**パターンをトライに事前コンパイル**（`pattern_match`を索引化）→ DPOは整合/生成に集中。
* **大きなボディ**：Graphにはハッシュ/長さのみ。Blobは**メモリマップ/零コピー**で返す。
* **安全モード**：`exhaust`連発は禁止、**measure必須**で停止性を強制（本番OLTP）。
* **観測性**：`Request --LOG--> Event` をPatchで積み、**由来**（`(input_graph, rule_hash, plan_hash)`）をコミットに記録。

---

## まず動く最小ターゲット

* ルート：`GET /ping`（固定JSON）/ `GET /static/:file`（blob返却）
* ミドルウェア：`req-id` / `server` / `content-type`
* これで**1ワーカー=1グラフ分岐**のPoCが成立。次に**複数接続/分散**では、`Request`をシャードローカルに格納し、ログだけ集約。

---

小さなユーモア：

> “HTTP”も結局は**グラフに来てグラフから去るパケット**。路地（Route）で会わせて、茶（Middleware）を出し、土産（Response）を持たせて帰すだけです ☕️➡️📦

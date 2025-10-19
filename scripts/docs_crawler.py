#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
shiori (CLI) - 実装 ⇔ ドキュメントの乖離検知・在庫化・API抽出
=======================================================
個人開発向けの**完全オフライン**・**単体ファイル**のCLIツールです。
APIや外部サービスは呼びません（= 課金ゼロ）。

出力物（--out ディレクトリ、既定は ./.shiori）:
  - docs-inventory.csv       : ドキュメント一覧（パス/サイズ/見出し/リンク等）
  - api-surface.csv          : 公開APIサーフェス（言語/種別/名前/ファイルパス）
  - coverage.csv             : シンボルのドキュメント言及カバレッジ
  - drift_report.md          : 乖離レポート（追加/削除/未ドキュメント等）
  - snapshot.json            : 現在のスナップショット（比較用）
  - snapshots/snapshot_*.json

使い方:
  python scripts/docs_crawler.py --repo .
  python scripts/docs_crawler.py --repo . --out ./.shiori --langs py,ts,go --ignore node_modules,.git,dist

# 1) ファイル配置
#   リポジトリ直下に scripts/docs_crawler.py として保存

# 2) 実行（既定の出力先は ./.shiori）
python scripts/docs_crawler.py --repo .

# 3) 結果（フォルダ ./.shiori/）
#   - drift_report.md
#   - api-surface.csv
#   - coverage.csv
#   - docs-inventory.csv
#   - snapshot.json / snapshots/*

# TS/Go だけ対象にして、重いディレクトリを除外
python scripts/docs_crawler.py --repo . --langs ts,go --ignore node_modules,dist,.git

# 前回比較なし（現状の未言及だけ知りたい）
python scripts/docs_crawler.py --repo . --since-snapshot none

サポート言語（軽量ヒューリスティック）:
  Python / TypeScript / JavaScript / Go / Rust / Java / Kotlin

制限:
  - 100%の厳密さは保証しません（軽量な抽出・語句一致検索）。
  - 大規模リポジトリでは時間がかかる場合があります（--langs / --ignore を活用）。
"""
from __future__ import annotations
import argparse
import csv
import datetime as _dt
import hashlib
import io
import json
import os
import re
import sys
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Set, Tuple

DEFAULT_OUT = ".shiori"  # ← 既定の作業ディレクトリ。再読み込み対象から除外します。

DOC_EXTS = {".md", ".mdx", ".rst", ".adoc", ".txt"}
# 拡張子→言語
LANG_OF_EXT = {
    ".py": "python",
    ".ts": "ts",
    ".tsx": "ts",
    ".js": "js",
    ".jsx": "js",
    ".go": "go",
    ".rs": "rust",
    ".java": "java",
    ".kt": "kotlin",
}
SUPPORTED_LANGS = {"python","ts","js","go","rust","java","kotlin"}

DEFAULT_IGNORE_DIRS = {
    ".git","node_modules","dist","build",".venv","venv","target","out",
    "coverage","__pycache__", ".idea", ".vscode", DEFAULT_OUT
}

# --------------------- 基本ユーティリティ ---------------------
def now_iso() -> str:
    return _dt.datetime.utcnow().replace(microsecond=0).isoformat() + "Z"

def sha1_of(path: Path) -> str:
    h = hashlib.sha1()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()

def is_binary(path: Path, blocksize: int = 512) -> bool:
    try:
        with open(path, "rb") as f:
            return b"\0" in f.read(blocksize)
    except Exception:
        return True

def walk_files(root: Path, ignore_dirs: Set[str]) -> Iterable[Path]:
    ignore = set(ignore_dirs)
    for current_root, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d not in ignore]
        cur_path = Path(current_root)
        for name in files:
            if name in ignore:
                continue
            yield cur_path / name

def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return ""

def relpath(root: Path, p: Path) -> str:
    try:
        return str(p.relative_to(root))
    except Exception:
        return str(p)

# --------------------- ドキュメントの在庫化 ---------------------
def extract_headings_md_like(text: str) -> List[str]:
    heads: List[str] = []
    for line in text.splitlines():
        s = line.lstrip()
        if s.startswith("#"):
            heads.append(s.lstrip("# ").strip())
    return heads

def extract_links_md_like(text: str) -> List[str]:
    links = re.findall(r"\[(?:[^\]]+)\]\(([^)]+)\)", text)  # markdown link
    links += re.findall(r"`([^`]+)`", text)                 # inline code reference
    # 簡易: 重複除去 & 並べ替え
    return sorted(set(links))

def sanitize_docs_for_output(docs: List[Dict[str, object]]) -> List[Dict[str, object]]:
    sanitized: List[Dict[str, object]] = []
    for doc in docs:
        entry = {
            k: (list(v) if isinstance(v, list) else v)
            for k, v in doc.items() if k != "text"
        }
        sanitized.append(entry)
    return sanitized

def discover_docs(repo: Path, ignore_dirs: Set[str]) -> List[Dict[str, object]]:
    docs: List[Dict[str, object]] = []
    for p in walk_files(repo, ignore_dirs):
        if p.suffix.lower() in DOC_EXTS and not is_binary(p):
            txt = read_text(p)
            headings = extract_headings_md_like(txt)
            links = extract_links_md_like(txt)
            docs.append({
                "path": relpath(repo, p),
                "sha1": sha1_of(p),
                "size": p.stat().st_size,
                "mtime": int(p.stat().st_mtime),
                "headings": headings,
                "num_headings": len(headings),
                "links": links,
                "num_links": len(links),
                "text": txt,
            })
    return docs

# --------------------- API 抽出（軽量） ---------------------
def detect_lang_of_file(p: Path) -> Optional[str]:
    return LANG_OF_EXT.get(p.suffix.lower())

def extract_symbols_from_code(txt: str, lang: str, path_rel: str) -> Iterable[Dict[str, str]]:
    """
    公開APIっぽいものを抽出（軽量ヒューリスティック）。
    - Python: トップレベルの public class/def（先頭 '_' を除外）
    - TS/JS:  export function/class/const/type/interface
    - Go:     先頭大文字の func / type
    - Rust:   pub fn/struct/enum/trait
    - Java/Kt: public class/interface/enum
    """
    if not txt:
        return []

    if lang == "python":
        import ast
        try:
            t = ast.parse(txt)
        except Exception:
            return []
        for node in t.body:
            if getattr(node, "name", None) and not node.name.startswith("_"):
                if node.__class__.__name__ == "FunctionDef":
                    yield {"lang": lang, "kind": "function", "name": node.name, "path": path_rel}
                elif node.__class__.__name__ == "ClassDef":
                    yield {"lang": lang, "kind": "class", "name": node.name, "path": path_rel}
        return

    if lang in ("ts", "js"):
        patterns = [
            r"export\s+(?:default\s+)?(?:async\s+)?function\s+(\w+)",
            r"export\s+class\s+(\w+)",
            r"export\s+interface\s+(\w+)",
            r"export\s+type\s+(\w+)",
            r"export\s+const\s+(\w+)",
            r"export\s+function\s+(\w+)",
        ]
        names: Set[str] = set()
        for pat in patterns:
            for m in re.finditer(pat, txt):
                names.add(m.group(1))
        for name in sorted(names):
            # 種別は簡易に "symbol" とする（必要なら詳細化）
            yield {"lang": lang, "kind": "symbol", "name": name, "path": path_rel}
        return

    if lang == "go":
        for m in re.finditer(r"func\s+([A-Z]\w*)\s*\(", txt):
            yield {"lang": lang, "kind": "function", "name": m.group(1), "path": path_rel}
        for m in re.finditer(r"type\s+([A-Z]\w*)\s+", txt):
            yield {"lang": lang, "kind": "type", "name": m.group(1), "path": path_rel}
        return

    if lang == "rust":
        for m in re.finditer(r"pub\s+(?:async\s+)?fn\s+(\w+)\s*\(", txt):
            yield {"lang": lang, "kind": "function", "name": m.group(1), "path": path_rel}
        for m in re.finditer(r"pub\s+struct\s+(\w+)", txt):
            yield {"lang": lang, "kind": "struct", "name": m.group(1), "path": path_rel}
        for m in re.finditer(r"pub\s+enum\s+(\w+)", txt):
            yield {"lang": lang, "kind": "enum", "name": m.group(1), "path": path_rel}
        for m in re.finditer(r"pub\s+trait\s+(\w+)", txt):
            yield {"lang": lang, "kind": "trait", "name": m.group(1), "path": path_rel}
        return

    if lang in ("java", "kotlin"):
        for m in re.finditer(r"public\s+(?:abstract\s+|final\s+)?(class|interface|enum)\s+(\w+)", txt):
            kind = m.group(1)
            name = m.group(2)
            yield {"lang": lang, "kind": kind, "name": name, "path": path_rel}
        return

    return []

def extract_api_surface(repo: Path, target_langs: Set[str], ignore_dirs: Set[str]) -> List[Dict[str, str]]:
    items: List[Dict[str, str]] = []
    for p in walk_files(repo, ignore_dirs):
        lang = detect_lang_of_file(p)
        if not lang or lang not in target_langs:
            continue
        if is_binary(p):
            continue
        txt = read_text(p)
        for entry in extract_symbols_from_code(txt, lang, relpath(repo, p)):
            items.append(entry)
    return items

def build_symbol_index(symbols: List[Dict[str, str]]) -> Dict[str, Dict[str, object]]:
    idx: Dict[str, Dict[str, object]] = {}
    for s in symbols:
        key = f"{s['lang']}:{s['name']}"
        rec = idx.setdefault(key, {"name": s["name"], "lang": s["lang"], "paths": set()})
        rec["paths"].add(s["path"])
    # set→list
    for k, v in idx.items():
        v["paths"] = sorted(v["paths"])
    return idx

def compute_doc_mentions(docs: List[Dict[str, object]], sym_idx: Dict[str, Dict[str, object]]) -> Dict[str, Set[str]]:
    mentions: Dict[str, Set[str]] = {k: set() for k in sym_idx.keys()}
    for d in docs:
        path_str = str(d["path"])
        txt = d.get("text") if isinstance(d, dict) else ""
        if not isinstance(txt, str):
            txt = ""
        for key, info in sym_idx.items():
            name = info["name"]
            if not name:
                continue
            # 単語境界・軽量
            if re.search(rf"\b{re.escape(name)}\b", txt):
                mentions[key].add(path_str)
    return mentions

# --------------------- ドリフト（スナップショット比較） ---------------------
def load_latest_snapshot(outdir: Path) -> Optional[Dict[str, object]]:
    snaps = sorted((outdir / "snapshots").glob("snapshot_*.json"))
    if not snaps:
        return None
    return json.loads(snaps[-1].read_text(encoding="utf-8"))

def save_snapshot(outdir: Path, snapshot: Dict[str, object]) -> Path:
    (outdir / "snapshots").mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.utcnow().strftime("%Y%m%d_%H%M%S")
    path = outdir / "snapshots" / f"snapshot_{ts}.json"
    path.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2), encoding="utf-8")
    (outdir / "snapshot.json").write_text(json.dumps(snapshot, ensure_ascii=False, indent=2), encoding="utf-8")
    return path

def compute_drift(prev: Optional[Dict[str, object]], sym_idx: Dict[str, Dict[str, object]], docs: List[Dict[str, object]]) -> Dict[str, object]:
    cur_keys = set(sym_idx.keys())
    if prev and "symbols" in prev:
        prev_keys = set(prev["symbols"].keys())  # type: ignore[assignment]
    else:
        prev_keys = set()

    added = sorted(cur_keys - prev_keys)
    removed = sorted(prev_keys - cur_keys)

    # 未ドキュメント
    mentions = compute_doc_mentions(docs, sym_idx)
    undocumented = sorted([k for k, v in mentions.items() if len(v) == 0])

    orphan_docs: List[Dict[str, str]] = []
    doc_text_lookup = {str(d["path"]): d.get("text") if isinstance(d, dict) else "" for d in docs}
    if prev and "symbols" in prev:
        removed_names = {prev["symbols"][k]["name"] for k in removed}  # type: ignore[index]
        for d in docs:
            txt = doc_text_lookup.get(str(d["path"]), "")
            if not isinstance(txt, str):
                txt = ""
            for name in removed_names:
                if re.search(rf"\b{re.escape(str(name))}\b", txt):
                    orphan_docs.append({"doc": str(d["path"]), "name": str(name)})

    return {
        "added_symbols": added,
        "removed_symbols": removed,
        "undocumented_symbols": undocumented,
        "mentions": {k: sorted(v) for k, v in mentions.items()},
        "orphan_doc_refs": orphan_docs,
    }

# --------------------- 出力 ---------------------
def write_csv(path: Path, rows: List[Dict[str, object]], fieldnames: Sequence[str]) -> None:
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=fieldnames)
        w.writeheader()
        for r in rows:
            w.writerow(r)

def write_reports(outdir: Path, repo: Path, docs: List[Dict[str, object]], symbols: List[Dict[str, str]], sym_idx: Dict[str, Dict[str, object]], drift: Dict[str, object]) -> None:
    # docs-inventory.csv
    write_csv(outdir / "docs-inventory.csv", docs,
              ["path","size","num_headings","num_links","sha1","mtime"])

    # api-surface.csv
    write_csv(outdir / "api-surface.csv", symbols,
              ["lang","kind","name","path"])

    # coverage.csv
    cov_rows: List[Dict[str, object]] = []
    mentions: Dict[str, List[str]] = drift["mentions"]  # type: ignore[assignment]
    for k, info in sym_idx.items():
        cov_rows.append({
            "symbol_key": k,
            "name": info["name"],
            "lang": info["lang"],
            "paths": ";".join(info["paths"]),  # type: ignore[index]
            "mentioned_in_docs": ";".join(mentions.get(k, [])),
            "mentioned_count": len(mentions.get(k, [])),
        })
    write_csv(outdir / "coverage.csv", cov_rows,
              ["symbol_key","name","lang","paths","mentioned_in_docs","mentioned_count"])

    # drift_report.md
    lines: List[str] = []
    lines.append(f"# Drift Report ({now_iso()})")
    lines.append("")
    lines.append(f"- Repository: `{repo}`")
    lines.append(f"- Docs files: {len(docs)}")
    lines.append(f"- Public symbols: {len(sym_idx)}")
    lines.append("")
    lines.append("## Undocumented symbols")
    und = drift.get("undocumented_symbols", [])
    if und:
        for k in und: lines.append(f"- {k}")
    else:
        lines.append("(none)")
    lines.append("")
    lines.append("## Added symbols since previous snapshot")
    add = drift.get("added_symbols", [])
    if add:
        for k in add: lines.append(f"- {k}")
    else:
        lines.append("(n/a)")
    lines.append("")
    lines.append("## Removed symbols since previous snapshot")
    rem = drift.get("removed_symbols", [])
    if rem:
        for k in rem: lines.append(f"- {k}")
    else:
        lines.append("(n/a)")
    lines.append("")
    lines.append("## Docs referencing removed symbols")
    orphans = drift.get("orphan_doc_refs", [])
    if orphans:
        for o in orphans:
            lines.append(f"- {o['doc']}: {o['name']}")
    else:
        lines.append("(n/a)")

    (outdir / "drift_report.md").write_text("\n".join(lines), encoding="utf-8")

# --------------------- メイン ---------------------
def main(argv: Optional[Sequence[str]] = None) -> int:
    ap = argparse.ArgumentParser(description="実装⇔ドキュメント乖離検知 CLI（オフライン）")
    ap.add_argument("--repo", type=str, default=".", help="スキャン対象のリポジトリルート")
    ap.add_argument("--out", type=str, default=DEFAULT_OUT, help="出力ディレクトリ（既定: ./.shiori）")
    ap.add_argument("--langs", type=str, default="all", help="対象言語（例: py,ts,go / 既定: all）")
    ap.add_argument("--ignore", type=str, default=",".join(sorted(DEFAULT_IGNORE_DIRS)), help="無視するディレクトリ名(カンマ区切り)")
    ap.add_argument("--since-snapshot", type=str, default="latest", help="'latest' or スナップショットjsonへのパス or 'none'")
    args = ap.parse_args(argv)

    repo = Path(args.repo).resolve()
    outdir = Path(args.out).resolve()
    outdir.mkdir(parents=True, exist_ok=True)

    ignore_dirs = set([x.strip() for x in args.ignore.split(",") if x.strip()])
    ignore_dirs.add(DEFAULT_OUT)
    out_path = Path(args.out)
    if out_path.name:
        ignore_dirs.add(out_path.name)
    # 言語
    if args.langs.lower() == "all":
        target_langs = set(SUPPORTED_LANGS)
    else:
        m = {
            "py":"python","python":"python",
            "ts":"ts","tsx":"ts",
            "js":"js","jsx":"js",
            "go":"go",
            "rs":"rust","rust":"rust",
            "java":"java","kt":"kotlin","kotlin":"kotlin"
        }
        target_langs = set()
        for token in args.langs.split(","):
            token = token.strip().lower()
            if token in m:
                target_langs.add(m[token])
        if not target_langs:
            print("対象言語が空です (--langs)。例: --langs py,ts,go", file=sys.stderr)
            return 2

    # 在庫化
    docs_raw = discover_docs(repo, ignore_dirs)
    # API抽出
    symbols = extract_api_surface(repo, target_langs, ignore_dirs)
    sym_idx = build_symbol_index(symbols)
    docs_sanitized = sanitize_docs_for_output(docs_raw)

    # 旧スナップショット読み込み
    prev = None
    if args.since_snapshot == "latest":
        prev = load_latest_snapshot(outdir)
    elif args.since_snapshot == "none":
        prev = None
    else:
        p = Path(args.since_snapshot)
        if p.exists():
            prev = json.loads(p.read_text(encoding="utf-8"))

    # ドリフト
    drift = compute_drift(prev, sym_idx, docs_raw)

    # 出力
    write_reports(outdir, repo, docs_sanitized, symbols, sym_idx, drift)

    # スナップショット保存
    snapshot = {
        "generated_at": now_iso(),
        "symbols": sym_idx,
        "docs_count": len(docs_raw),
        "docs": docs_sanitized,
        "repo": str(repo),
        "target_langs": sorted(target_langs),
        "ignore_dirs": sorted(ignore_dirs),
    }
    save_snapshot(outdir, snapshot)

    # 端末表示
    print("✅ 完了")
    print(f"  Repo: {repo}")
    print(f"  Docs: {len(docs_raw)} files")
    print(f"  Symbols: {len(sym_idx)} unique")
    print(f"  Out: {outdir}")
    print(f"  Report: {(outdir/'drift_report.md')}")

    return 0

if __name__ == "__main__":
    raise SystemExit(main())

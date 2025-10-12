#!/usr/bin/env bash
# ADR採番検証スクリプト
# Umbrella spec全体でADR採番の重複と欠番をチェック
# Requires: bash 4+ (for associative arrays)

set -euo pipefail

# Check bash version
if ((BASH_VERSINFO[0] < 4)); then
  echo "⚠️  This script requires bash 4+. Using fallback mode..."
  LEGACY_MODE=true
else
  LEGACY_MODE=false
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SPECS_DIR="$PROJECT_ROOT/.kiro/specs"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 ADR採番検証を開始..."
echo ""

# すべてのADRファイルを検索
adr_files=$(find "$SPECS_DIR" -path "*/adrs/ADR-*.md" | sort)

if [ -z "$adr_files" ]; then
  echo -e "${YELLOW}⚠️  ADRファイルが見つかりません${NC}"
  exit 0
fi

# ADR番号を抽出（レガシーモード対応）
if [ "$LEGACY_MODE" = false ]; then
  declare -A adr_map  # 連想配列: ADR番号 → ファイルパス
fi
declare -a adr_numbers  # 配列: すべてのADR番号
declare -a adr_files_array  # ファイルパス配列（レガシーモード用）

while IFS= read -r file; do
  # ファイル名からADR番号を抽出 (例: ADR-001-xxx.md → 001)
  adr_num=$(basename "$file" | sed -E 's/ADR-([0-9]+)-.*/\1/')

  # 先頭ゼロを除去して整数化 (001 → 1)
  adr_int=$((10#$adr_num))

  if [ "$LEGACY_MODE" = false ]; then
    # 連想配列に追加（重複チェック用）
    if [ -n "${adr_map[$adr_int]:-}" ]; then
      echo -e "${RED}❌ ADR-$(printf '%03d' $adr_int) 重複検出:${NC}"
      echo "   1. ${adr_map[$adr_int]}"
      echo "   2. $file"
      echo ""
      exit 1
    fi
    adr_map[$adr_int]="$file"
  else
    # レガシーモード: 配列で重複チェック（線形探索）
    for i in "${!adr_numbers[@]}"; do
      if [ "${adr_numbers[$i]}" = "$adr_int" ]; then
        echo -e "${RED}❌ ADR-$(printf '%03d' $adr_int) 重複検出:${NC}"
        echo "   1. ${adr_files_array[$i]}"
        echo "   2. $file"
        echo ""
        exit 1
      fi
    done
    adr_files_array+=("$file")
  fi

  adr_numbers+=($adr_int)
done <<< "$adr_files"

# ソート
IFS=$'\n' sorted=($(sort -n <<<"${adr_numbers[*]}"))
unset IFS

echo -e "${GREEN}✅ 重複なし${NC}"
echo ""

# 欠番チェック
echo "📋 検出されたADR:"
# Get last element (bash 3 compatible)
max_adr=${sorted[${#sorted[@]}-1]}
missing=()

for ((i=1; i<=max_adr; i++)); do
  found=false
  file_path=""

  if [ "$LEGACY_MODE" = false ]; then
    if [ -n "${adr_map[$i]:-}" ]; then
      found=true
      file_path="${adr_map[$i]}"
    fi
  else
    # レガシーモード: 線形探索
    for j in "${!adr_numbers[@]}"; do
      if [ "${adr_numbers[$j]}" = "$i" ]; then
        found=true
        file_path="${adr_files_array[$j]}"
        break
      fi
    done
  fi

  if [ "$found" = true ]; then
    # sub-spec名を抽出 (例: .kiro/specs/meeting-minutes-stt/adrs/... → stt)
    spec_name=$(echo "$file_path" | sed -E 's|.*meeting-minutes-([^/]+)/adrs/.*|\1|')
    echo "   ADR-$(printf '%03d' $i): $spec_name"
  else
    missing+=($i)
  fi
done

echo ""

if [ ${#missing[@]} -gt 0 ]; then
  echo -e "${YELLOW}⚠️  欠番検出:${NC}"
  for num in "${missing[@]}"; do
    echo "   ADR-$(printf '%03d' $num)"
  done
  echo ""
  echo -e "${YELLOW}注: 欠番は必ずしもエラーではありませんが、意図的か確認してください${NC}"
else
  echo -e "${GREEN}✅ 欠番なし (ADR-001 〜 ADR-$(printf '%03d' $max_adr))${NC}"
fi

echo ""
echo -e "${GREEN}🎉 ADR採番検証完了${NC}"

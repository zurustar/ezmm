#!/bin/bash
# 設計レビューの状態をチェックし、未解決項目があれば警告、0 件なら再レビューを促すメッセージを context に注入する
# docs/tasks.md の「N回目レビューで発見した追加ギャップ」セクション内の - [ ] 項目を数える

set -euo pipefail

TASKS_FILE="$(dirname "$0")/../../docs/tasks.md"

if [ ! -f "$TASKS_FILE" ]; then
  exit 0
fi

# レビューセクション内の未解決項目を数える
UNCHECKED=$(awk '
  /^## .*回目レビューで発見した追加ギャップ/ { in_review=1; next }
  /^## / && in_review { in_review=0 }
  /^---$/ && in_review { in_review=0 }
  in_review && /^- \[ \]/ { count++ }
  END { print count+0 }
' "$TASKS_FILE")

if [ "$UNCHECKED" -gt 0 ]; then
  MESSAGE="⚠️ 設計レビュー未解決: docs/tasks.md に未解決の設計ギャップが ${UNCHECKED} 件残っています。次を厳守してください: (1)「設計完了」「製造に進める」「実装タスクに着手可能」などと宣言しない (2) すべての項目が [x] になるまで「徹底レビュー → 新規ギャップ書き起こし → 解決」のループを継続する (3) レビューの具体的な手順は /design-review コマンドを使う (4) 1件でも未解決があればこの警告が出続けます"
else
  MESSAGE="ℹ️ 設計レビューの未解決項目は 0 件です。ただし「0 件 = 設計完了」ではありません。/design-review コマンドで新しいレビューを実施し、新規課題が見つからないかを確認してから、初めて次フェーズへ進めるか判断してください。"
fi

# JSON エスケープ（制御文字・クォート・バックスラッシュを安全に処理）
ESCAPED=$(printf '%s' "$MESSAGE" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')

cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": ${ESCAPED}
  }
}
EOF

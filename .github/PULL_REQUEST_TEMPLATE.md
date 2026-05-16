<!--
Thanks for the contribution! Please read CONTRIBUTING.md before opening a PR.
Skip sections that don't apply.
-->

## Summary

<!-- 何を、なぜ変更したかを 1〜3 文で。背景があれば issue へリンク。 -->

## Related issues

<!-- Closes #123 / Refs #456 のように記載 (なければ削除可) -->

## Type of change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation / housekeeping (no code behaviour change)

## Testing

<!-- どうやって検証したかを書く。手動手順 / 追加した自動テストの種類 / etc. -->

## Pre-submit checklist

- [ ] `bash scripts/verify-gate.sh` がローカルで緑 (cargo fmt / clippy -D warnings / cargo test --lib / prettier / vue-tsc / i18n parity / vitest)
- [ ] UI 文言を追加/変更した場合は `app/locales/ja.ts` と `app/locales/en.ts` のキー数が一致する (`node scripts/check-i18n.mjs` 緑)
- [ ] 新規 IPC コマンド / モジュール追加時は `docs/architecture.md` と `docs/file_inventory.md` を同期した
- [ ] ユーザー可視動作 / インストールフロー / セキュリティモデルに影響する変更は `README.md` / `README.ja.md` 両方と `CHANGELOG.md`(`[Unreleased]`) を更新した
- [ ] `Co-Authored-By` 行を含むコミットは AI コラボの場合のみ付与 (任意)

## Notes for reviewer

<!-- レビュアー向けの補足。確認してほしい観点、ベンチマーク結果、スクリーンショット等 -->

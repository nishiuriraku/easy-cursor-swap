<!--
  鍵ローテーション PR テンプレート (`easycursorswap/index` リポジトリ用の雛形)。
  本リポジトリでは参照のために配置。`easycursorswap/index` 側にコピーして使用する。

  使用方法:
    https://github.com/easycursorswap/index/compare?template=key_rotation.md
-->

## 鍵ローテーション

- **種別**: [ ] 定期更新 / [ ] 緊急 (漏洩疑い)
- **GitHub ユーザー**: @<!-- your-username -->
- **旧 key_id**: `<!-- 16 文字 hex -->`
- **新 key_id**: `<!-- 16 文字 hex -->`
- **historical_keys に旧鍵を残す**: [ ] はい (推奨・通常更新) / [ ] いいえ (緊急失効)

## 動機 (任意)

<!--
  例:
  - 12 か月経過したため定期更新
  - 古い端末を破棄したため
  - 秘密鍵を含むファイルを誤公開した恐れがあるため緊急失効
-->

## チェックリスト

- [ ] 新鍵で EasyCursorSwap アプリの `keystore_info` を呼び `key_id` が一致することを確認した
- [ ] 旧 `public_key` を `historical_keys` に正しい `key_id` で登録した (緊急失効でない場合)
- [ ] 秘密鍵をコミットしていない (`.gitignore` で除外されている)
- [ ] `authors/<github_username>.json` のスキーマが docs/key_rotation.md と一致
- [ ] (緊急時のみ) 旧鍵で署名済みのテーマを再署名して別 PR で提出予定がある

## 詳細手順

参照: [docs/key_rotation.md](https://github.com/easycursorswap/easy-cursor-swap/blob/main/docs/key_rotation.md)

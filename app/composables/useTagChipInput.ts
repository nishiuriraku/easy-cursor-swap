/**
 * "Tag chip" 入力 UI のロジックを集約する composable。
 *
 * SubmitThemeDialog の Auto / Manual タブの双方で `tags` / `tagInput` /
 * `commitTag` / `onTagKeydown` / `removeTag` の同一セットが使われており、
 * 将来別の form でも tag 入力が必要になる可能性が高いため抽出する。
 *
 * 受け付ける入力:
 *   - Enter / カンマ / セミコロンで chip 化 (複数文字列をコンマで同時投入可能)
 *   - Backspace (input 空の場合) で最後の chip を削除
 *   - 重複は弾く・全 chip を `toLowerCase()` 化
 *
 * 上限:
 *   - chips 個数 (default 8)
 *   - 1 chip あたりの最大長 (default 24)
 */

export interface TagChipInputOptions {
  /** タグ chip の最大個数 (default 8) */
  maxTags?: number
  /** 1 chip の最大文字数 (default 24) */
  maxTagLen?: number
}

export function useTagChipInput(opts: TagChipInputOptions = {}) {
  const maxTags = opts.maxTags ?? 8
  const maxTagLen = opts.maxTagLen ?? 24

  const tagInput = ref('')
  const tags = ref<string[]>([])

  function commitTag(): void {
    const raw = tagInput.value.trim()
    if (!raw) return
    const parts = raw
      .split(/[,;]/)
      .map((s) => s.trim().toLowerCase())
      .filter((s) => s.length > 0 && s.length <= maxTagLen)
    for (const p of parts) {
      if (tags.value.length >= maxTags) break
      if (!tags.value.includes(p)) tags.value.push(p)
    }
    tagInput.value = ''
  }

  function onTagKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' || e.key === ',' || e.key === ';') {
      e.preventDefault()
      commitTag()
    } else if (e.key === 'Backspace' && tagInput.value === '' && tags.value.length > 0) {
      tags.value.pop()
    }
  }

  function removeTag(i: number): void {
    tags.value.splice(i, 1)
  }

  function reset(): void {
    tags.value = []
    tagInput.value = ''
  }

  return { tagInput, tags, commitTag, onTagKeydown, removeTag, reset, maxTags, maxTagLen }
}

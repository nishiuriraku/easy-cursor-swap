/**
 * Rust `AppError` の IPC 表現 (`AppErrorDto { code, message, detail? }`) を
 * フロントで受け直すための正規化層 (P03 Step 3)。
 *
 * `invokeTauri` の catch で `throw toAppError(err)` し、呼び出し側は
 * `appErrorMessage(err)` で表示文言を得る。`message` は Rust の Display と
 * 同一なので、既存の `err.message` 直参照も壊れない。
 */
import type { AppErrorCode, AppErrorDto } from '~/types/generated'

/** Rust `AppError` を invoke 境界で受け直した Error。`message` は Rust の Display と同一。 */
export class AppInvokeError extends Error {
  readonly code: AppErrorCode
  readonly detail: Readonly<Record<string, string>>
  /** 元の reject 値 (デバッグ用) */
  readonly raw: unknown
  constructor(dto: AppErrorDto, raw: unknown) {
    super(dto.message)
    this.name = 'AppInvokeError'
    this.code = dto.code
    this.detail = Object.freeze({ ...(dto.detail ?? {}) }) as Record<string, string>
    this.raw = raw
  }
}

function isDto(v: unknown): v is AppErrorDto {
  return (
    typeof v === 'object' &&
    v !== null &&
    typeof (v as AppErrorDto).code === 'string' &&
    typeof (v as AppErrorDto).message === 'string'
  )
}

/** 何が飛んできても `AppInvokeError` に正規化する (旧形式の文字列 / Error / 不明値は code 'other')。 */
export function toAppError(err: unknown): AppInvokeError {
  if (err instanceof AppInvokeError) return err
  if (isDto(err)) return new AppInvokeError(err, err)
  if (err instanceof Error) return new AppInvokeError({ code: 'other', message: err.message }, err)
  if (typeof err === 'string') return new AppInvokeError({ code: 'other', message: err }, err)
  return new AppInvokeError({ code: 'other', message: String(err) }, err)
}

/** 表示用文言。`errors.<code>` があれば言語別文言 (message / detail を補間)、無ければ Rust の message。 */
export function appErrorMessage(err: unknown): string {
  const e = toAppError(err)
  const { t, te } = useI18n()
  const key = `errors.${e.code}`
  return te(key) ? t(key, { message: e.message, ...e.detail }) : e.message
}

const JINA_READER_URL = 'https://r.jina.ai/'
const WINDOW_MS = 60_000
const MAX_USES_PER_WINDOW = 3
const REQUEST_TIMEOUT_MS = 20_000
const SUMMARY_MAX_LENGTH = 3_500

type JinaPayload = {
    title?: unknown
    description?: unknown
    content?: unknown
    url?: unknown
    data?: JinaPayload | null
}

export type ResumeResult = {
    title: string
    sourceUrl: string
    summary: string
}

export type ResumeRateLimitResult = {
    allowed: boolean
    remaining: number
    retryAfterMs?: number
}

export class ResumeContentError extends Error {}

const usageByUser = new Map<string, number[]>()

export function parseResumeUrl(value: string): string {
    let url: URL

    try {
        url = new URL(value.trim())
    } catch {
        throw new Error('Informe uma URL válida, incluindo `https://`.')
    }

    if (!['http:', 'https:'].includes(url.protocol)) {
        throw new Error('A URL precisa usar o protocolo `http://` ou `https://`.')
    }

    if (url.hostname.toLowerCase() === 'r.jina.ai') {
        throw new Error('Informe o site original, não uma URL do Reader da Jina.')
    }

    return url.toString()
}

export function consumeResumeUse(userId: string): ResumeRateLimitResult {
    const now = Date.now()
    const recentUses = (usageByUser.get(userId) ?? []).filter((timestamp) => now - timestamp < WINDOW_MS)

    if (recentUses.length >= MAX_USES_PER_WINDOW) {
        usageByUser.set(userId, recentUses)
        return {
            allowed: false,
            remaining: 0,
            retryAfterMs: WINDOW_MS - (now - recentUses[0]),
        }
    }

    recentUses.push(now)
    usageByUser.set(userId, recentUses)

    return {
        allowed: true,
        remaining: MAX_USES_PER_WINDOW - recentUses.length,
    }
}

export async function summarizeUrl(sourceUrl: string): Promise<ResumeResult> {
    const rawResponse = await requestJina(sourceUrl)
    const payload = parseJinaResponse(rawResponse)
    const content = payload.content.trim()
    const title = cleanTitle(payload.title, sourceUrl)

    if (!content) {
        throw new ResumeContentError('A Jina não retornou conteúdo legível para esse site.')
    }

    if (looksLikeUnavailablePage(title, payload.description, content)) {
        throw new ResumeContentError('Essa URL parece estar indisponível ou retornar uma página 404. Verifique se o endereço está correto e se o conteúdo é público.')
    }

    const markdown = createReadableExcerpt(content)
    if (!markdown) {
        throw new ResumeContentError('Não encontrei conteúdo principal suficiente para resumir essa página.')
    }

    const canonicalUrl = typeof payload.url === 'string' && /^https?:\/\//i.test(payload.url)
        ? payload.url
        : sourceUrl

    return {
        title,
        sourceUrl: canonicalUrl,
        summary: markdown,
    }
}

async function requestJina(sourceUrl: string): Promise<string> {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS)

    try {
        const response = await fetch(`${JINA_READER_URL}${sourceUrl}`, {
            headers: {
                Accept: 'application/json',
                'User-Agent': 'LarperBot/1.0 (Jina Reader)',
                'X-Remove-Selector': 'header, nav, footer, aside, script, style, noscript, [role="navigation"], [role="banner"], [role="contentinfo"], .sidebar, .navbar, .cookie-banner, .advertisement, .ads',
            },
            signal: controller.signal,
        })

        if (!response.ok) {
            throw new Error(`Jina respondeu com HTTP ${response.status}.`)
        }

        return await response.text()
    } finally {
        clearTimeout(timeout)
    }
}

function parseJinaResponse(rawResponse: string): { title?: string; description?: string; content: string; url?: string } {
    try {
        const parsed = JSON.parse(rawResponse) as JinaPayload | string
        if (typeof parsed === 'string') return { content: decodeMarkdownString(parsed) }
        if (parsed.data === null) return { content: '' }

        const payload = parsed.data ?? parsed
        return {
            title: typeof payload.title === 'string' ? payload.title : undefined,
            description: typeof payload.description === 'string' ? payload.description : undefined,
            content: typeof payload.content === 'string' ? decodeMarkdownString(payload.content) : '',
            url: typeof payload.url === 'string' ? payload.url : undefined,
        }
    } catch {
        return { content: decodeMarkdownString(rawResponse) }
    }
}

function decodeMarkdownString(value: string): string {
    return value.includes('\\n') ? value.replaceAll('\\n', '\n') : value
}

function cleanTitle(title: string | undefined, sourceUrl: string): string {
    if (title?.trim()) return title.trim().slice(0, 256)

    try {
        return new URL(sourceUrl).hostname.slice(0, 256)
    } catch {
        return 'Site resumido'
    }
}

function looksLikeUnavailablePage(title: string, description: string | undefined, content: string): boolean {
    const text = `${title}\n${description ?? ''}\n${content}`.toLowerCase()
    const signals = [
        'this is not the web page you are looking for',
        'page not found',
        'repository not found',
        'error 404',
        '404 not found',
        'there was an error while loading',
    ]

    return signals.some((signal) => text.includes(signal))
}

function createReadableExcerpt(content: string): string {
    const seen = new Set<string>()
    const lines: string[] = []

    for (const rawLine of content.replace(/\r/g, '').split('\n')) {
        const line = rawLine.trim()
        if (!line) {
            if (lines.length > 0 && lines.at(-1) !== '') lines.push('')
            continue
        }

        if (/^!\[[^\]]*\]\([^)]*\)$/u.test(line) || isBoilerplateLine(line)) continue

        const key = line.toLowerCase()
        if (seen.has(key)) continue
        seen.add(key)
        lines.push(line)
    }

    while (lines.at(-1) === '') lines.pop()
    return truncateMarkdown(formatMarkdownForDiscord(lines.join('\n')), SUMMARY_MAX_LENGTH)
}

function isBoilerplateLine(line: string): boolean {
    const linkCount = line.match(/\[[^\]]+\]\([^)]*\)/g)?.length ?? 0
    return /^(\[[^\]]+\]\([^)]*\)|\s*)?(skip to content|navigation menu|sign in|sign up|pricing|contact support|github status|view all (features|solutions|resources|topics|use cases)|reload to refresh your session|dismiss alert|cookie settings|accept cookies)$/iu.test(line)
        || /(arrow|chevron|open_in_new|content_copy|keyboard_arrow|more_horiz|visibility_off|menu_book|globe_book|travel_explore|attach_money|play_arrow)/iu.test(line)
        || linkCount >= 4
}

function formatMarkdownForDiscord(markdown: string): string {
    const lines = markdown.split('\n')
    const formatted: string[] = []

    for (let index = 0; index < lines.length; index += 1) {
        const line = lines[index]
        const nextLine = lines[index + 1]

        if (isTableRow(line) && nextLine && isTableSeparator(nextLine)) {
            const headers = splitTableRow(line)
            index += 1

            while (index + 1 < lines.length && isTableRow(lines[index + 1]) && !isTableSeparator(lines[index + 1])) {
                index += 1
                const cells = splitTableRow(lines[index])
                const row = cells
                    .map((cell, cellIndex) => {
                        const value = formatInlineMarkdown(cell)
                        const header = formatInlineMarkdown(headers[cellIndex] ?? '')
                        return value ? `${header ? `**${header}:** ` : ''}${value}` : ''
                    })
                    .filter(Boolean)
                    .join(' · ')

                if (row) formatted.push(`• ${row}`)
            }

            continue
        }

        formatted.push(formatInlineMarkdown(line))
    }

    return formatted
        .join('\n')
        .replace(/\n{3,}/g, '\n\n')
        .trim()
}

function isTableRow(line: string): boolean {
    return /^\s*\|.+\|\s*$/u.test(line)
}

function isTableSeparator(line: string): boolean {
    return /^\s*\|(?:\s*:?-{1,}:?\s*\|)+\s*$/u.test(line)
}

function splitTableRow(line: string): string[] {
    return line
        .trim()
        .replace(/^\||\|$/g, '')
        .split('|')
        .map((cell) => cell.trim())
}

function formatInlineMarkdown(value: string): string {
    return value
        .replace(/\[!\[([^\]]*)\]\((https?:\/\/[^)\s]+|blob:[^)]*)\)\]\((https?:\/\/[^)\s]+)\)/gu, (_match, alt: string, _imageUrl: string, targetUrl: string) => `[Imagem: ${alt || 'visual'}](${targetUrl})`)
        .replace(/!\[([^\]]*)\]\((https?:\/\/[^)\s]+|blob:[^)]*)\)/gu, (_match, alt: string, imageUrl: string) => imageUrl.startsWith('blob:') ? `*Imagem: ${alt || 'visual'}*` : `[Imagem: ${alt || 'visual'}](${imageUrl})`)
        .replace(/\[\[([^\]]+)\]\]\((https?:\/\/[^)\s]+)\)/gu, '[$1]($2)')
        .replace(/\\([\\`*_{}\[\]()#+.!|>~-])/gu, '$1')
}

function truncateMarkdown(value: string, maxLength: number): string {
    if (value.length <= maxLength) return value

    const boundary = value.lastIndexOf('\n\n', maxLength - 1)
    const end = boundary > Math.floor(maxLength * 0.6) ? boundary : maxLength - 1
    return `${value.slice(0, end).trimEnd()}…`
}

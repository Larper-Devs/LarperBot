export const DEVEX_USD_PER_ROBUX = 0.0038

const FRANKFURTER_API = 'https://api.frankfurter.dev/v2'
const CURRENCY_CACHE_TTL = 24 * 60 * 60 * 1_000
const RATE_CACHE_TTL = 15 * 60 * 1_000

type Currency = {
    code: string
    name: string
}

type CachedValue<T> = {
    expiresAt: number
    value: T
}

type RateResult = {
    date: string
    rate: number
}

const fallbackCurrencies: Currency[] = [
    ['AED', 'Dirham dos Emirados Árabes Unidos'],
    ['ARS', 'Peso argentino'],
    ['AUD', 'Dólar australiano'],
    ['BOB', 'Boliviano'],
    ['BRL', 'Real brasileiro'],
    ['CAD', 'Dólar canadense'],
    ['CHF', 'Franco suíço'],
    ['CLP', 'Peso chileno'],
    ['CNY', 'Yuan chinês'],
    ['COP', 'Peso colombiano'],
    ['CZK', 'Coroa tcheca'],
    ['DKK', 'Coroa dinamarquesa'],
    ['EUR', 'Euro'],
    ['GBP', 'Libra esterlina'],
    ['HKD', 'Dólar de Hong Kong'],
    ['HUF', 'Florim húngaro'],
    ['IDR', 'Rupia indonésia'],
    ['INR', 'Rupia indiana'],
    ['JPY', 'Iene japonês'],
    ['KRW', 'Won sul-coreano'],
    ['MXN', 'Peso mexicano'],
    ['MYR', 'Ringgit malaio'],
    ['NOK', 'Coroa norueguesa'],
    ['NZD', 'Dólar neozelandês'],
    ['PEN', 'Sol peruano'],
    ['PHP', 'Peso filipino'],
    ['PLN', 'Zloty polonês'],
    ['PYG', 'Guarani paraguaio'],
    ['RUB', 'Rublo russo'],
    ['SAR', 'Rial saudita'],
    ['SEK', 'Coroa sueca'],
    ['SGD', 'Dólar de Singapura'],
    ['THB', 'Baht tailandês'],
    ['TRY', 'Lira turca'],
    ['USD', 'Dólar americano'],
    ['UYU', 'Peso uruguaio'],
    ['VND', 'Dong vietnamita'],
    ['ZAR', 'Rand sul-africano'],
].map(([code, name]) => ({ code, name }))

let currenciesCache: CachedValue<Currency[]> | null = null
const ratesCache = new Map<string, CachedValue<RateResult>>()

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null
}

function parseCurrencies(payload: unknown): Currency[] {
    if (Array.isArray(payload)) {
        return payload.flatMap((item) => {
            if (!isRecord(item)) return []
            const code = item.code ?? item.currency ?? item.iso_code
            const name = item.name ?? item.currency_name ?? code
            if (typeof code !== 'string' || typeof name !== 'string') return []
            return [{ code: code.toUpperCase(), name }]
        })
    }

    if (isRecord(payload)) {
        return Object.entries(payload).flatMap(([code, name]) => (
            typeof name === 'string' ? [{ code: code.toUpperCase(), name }] : []
        ))
    }

    return []
}

async function requestJson(url: string): Promise<unknown> {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 5_000)

    try {
        const response = await fetch(url, { signal: controller.signal })
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        return await response.json() as unknown
    } finally {
        clearTimeout(timeout)
    }
}

export async function getCurrencies(): Promise<Currency[]> {
    if (currenciesCache && currenciesCache.expiresAt > Date.now()) return currenciesCache.value

    try {
        const currencies = parseCurrencies(await requestJson(`${FRANKFURTER_API}/currencies`))
        if (currencies.length > 0) {
            const value = currencies.sort((left, right) => left.code.localeCompare(right.code))
            currenciesCache = { value, expiresAt: Date.now() + CURRENCY_CACHE_TTL }
            return value
        }
    } catch {
        // O autocomplete continua disponível com a lista local se o serviço estiver indisponível.
    }

    currenciesCache = { value: fallbackCurrencies, expiresAt: Date.now() + 10 * 60 * 1_000 }
    return fallbackCurrencies
}

export async function searchCurrencies(query: string) {
    const normalizedQuery = query.trim().toLowerCase()
    const currencies = await getCurrencies()
    const filtered = currencies.filter((currency) => (
        !normalizedQuery
        || currency.code.toLowerCase().includes(normalizedQuery)
        || currency.name.toLowerCase().includes(normalizedQuery)
    ))

    return filtered.slice(0, 25).map((currency) => ({
        name: `${currency.code} — ${currency.name}`.slice(0, 100),
        value: currency.code,
    }))
}

async function getUsdRate(currency: string): Promise<RateResult> {
    if (currency === 'USD') return { date: new Date().toISOString().slice(0, 10), rate: 1 }

    const cached = ratesCache.get(currency)
    if (cached && cached.expiresAt > Date.now()) return cached.value

    const payload = await requestJson(`${FRANKFURTER_API}/rate/usd/${currency.toLowerCase()}`)
    if (!isRecord(payload) || typeof payload.rate !== 'number' || !Number.isFinite(payload.rate) || payload.rate <= 0) {
        throw new Error(`Não foi possível obter a cotação USD/${currency}.`)
    }

    const result: RateResult = {
        date: typeof payload.date === 'string' ? payload.date : new Date().toISOString().slice(0, 10),
        rate: payload.rate,
    }
    ratesCache.set(currency, { value: result, expiresAt: Date.now() + RATE_CACHE_TTL })
    return result
}

export async function calculateDevex(robux: number, currencyInput: string) {
    const currency = currencyInput.trim().toUpperCase()
    if (!/^[A-Z]{3}$/.test(currency)) throw new Error('Informe uma moeda no formato ISO 4217, como BRL ou USD.')
    if (!Number.isSafeInteger(robux) || robux < 1) throw new Error('A quantidade de Robux deve ser um número inteiro positivo.')

    const quote = await getUsdRate(currency)
    const usdAmount = robux * DEVEX_USD_PER_ROBUX

    return {
        currency,
        date: quote.date,
        rate: quote.rate,
        robux,
        usdAmount,
        localAmount: usdAmount * quote.rate,
    }
}

export function formatMoney(amount: number, currency: string): string {
    try {
        return new Intl.NumberFormat('pt-BR', {
            style: 'currency',
            currency,
        }).format(amount)
    } catch {
        return `${amount.toFixed(2)} ${currency}`
    }
}

export class CalculatorError extends Error {}

type Token =
    | { type: 'number', value: number }
    | { type: 'operator', value: '+' | '-' | '*' | '/' | '%' | '^' }
    | { type: 'parenthesis', value: '(' | ')' }

function tokenize(input: string): Token[] {
    const expression = input.replace(/,/g, '.').replace(/\s+/g, '')
    if (!expression) throw new CalculatorError('Informe uma expressão para calcular.')
    if (expression.length > 200) throw new CalculatorError('A expressão deve ter no máximo 200 caracteres.')

    const tokens: Token[] = []
    let index = 0

    while (index < expression.length) {
        const character = expression[index]

        if (character && /[0-9.]/.test(character)) {
            const start = index
            let dots = 0
            while (index < expression.length && /[0-9.]/.test(expression[index] ?? '')) {
                if (expression[index] === '.') dots += 1
                index += 1
            }

            const rawNumber = expression.slice(start, index)
            if (dots > 1 || rawNumber === '.') throw new CalculatorError(`Número inválido: ${rawNumber}`)
            const value = Number(rawNumber)
            if (!Number.isFinite(value)) throw new CalculatorError(`Número inválido: ${rawNumber}`)
            tokens.push({ type: 'number', value })
            continue
        }

        if (character && '+-*/%^'.includes(character)) {
            tokens.push({ type: 'operator', value: character as '+' | '-' | '*' | '/' | '%' | '^' })
            index += 1
            continue
        }

        if (character === '(' || character === ')') {
            tokens.push({ type: 'parenthesis', value: character })
            index += 1
            continue
        }

        throw new CalculatorError(`Caractere não permitido: ${character}`)
    }

    return tokens
}

class Parser {
    private readonly tokens: Token[]
    private index = 0

    constructor(input: string) {
        this.tokens = tokenize(input)
    }

    parse(): number {
        const result = this.parseAddSub()
        if (this.index < this.tokens.length) throw new CalculatorError('Expressão inválida ou parênteses incorretos.')
        if (!Number.isFinite(result)) throw new CalculatorError('O resultado não é um número finito.')
        return Object.is(result, -0) ? 0 : result
    }

    private current(): Token | undefined {
        return this.tokens[this.index]
    }

    private parseAddSub(): number {
        let result = this.parseMulDiv()

        while (true) {
            const token = this.current()
            if (token?.type !== 'operator' || (token.value !== '+' && token.value !== '-')) break
            this.index += 1
            const right = this.parseMulDiv()
            result = token.value === '+' ? result + right : result - right
        }

        return result
    }

    private parseMulDiv(): number {
        let result = this.parsePower()

        while (true) {
            const token = this.current()
            if (token?.type !== 'operator' || !['*', '/', '%'].includes(token.value)) break
            this.index += 1
            const right = this.parsePower()

            if ((token.value === '/' || token.value === '%') && right === 0) {
                throw new CalculatorError('Não é possível dividir por zero.')
            }

            if (token.value === '*') result *= right
            if (token.value === '/') result /= right
            if (token.value === '%') result %= right
        }

        return result
    }

    private parsePower(): number {
        const left = this.parseUnary()
        const token = this.current()
        if (token?.type !== 'operator' || token.value !== '^') return left

        this.index += 1
        const result = left ** this.parsePower()
        if (!Number.isFinite(result)) throw new CalculatorError('O resultado ficou grande demais.')
        return result
    }

    private parseUnary(): number {
        const token = this.current()
        if (token?.type === 'operator' && (token.value === '+' || token.value === '-')) {
            this.index += 1
            const value = this.parseUnary()
            return token.value === '-' ? -value : value
        }

        return this.parsePrimary()
    }

    private parsePrimary(): number {
        const token = this.current()
        if (!token) throw new CalculatorError('Faltou um número na expressão.')

        if (token.type === 'number') {
            this.index += 1
            return token.value
        }

        if (token.type === 'parenthesis' && token.value === '(') {
            this.index += 1
            const result = this.parseAddSub()
            const closing = this.current()
            if (closing?.type !== 'parenthesis' || closing.value !== ')') {
                throw new CalculatorError('Parênteses incorretos.')
            }
            this.index += 1
            return result
        }

        throw new CalculatorError('Esperava um número ou uma abertura de parênteses.')
    }
}

export function calculateExpression(input: string): number {
    return new Parser(input).parse()
}

export function formatCalculationResult(value: number): string {
    return new Intl.NumberFormat('pt-BR', {
        maximumFractionDigits: 10,
    }).format(value)
}

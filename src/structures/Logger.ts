import { red, blue, green, yellow, gray, bold, cyan } from 'colorette';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

export enum LogType {
  INFO,
  SUCCESS,
  WARN,
  UPDATE,
  ERROR,
  FATAL
}

export interface LogOptions {
  message?: string;
  includePath?: boolean;
  error?: unknown;
}

export class Logger {
  path: string;

  constructor(path: string = './src/logs/errorLog.log') {
    this.path = path;
  }

  public formatDate(date: Date = new Date()): string {
    const time = [date.getHours(), date.getMinutes(), date.getSeconds()]
      .map(n => n.toString().padStart(2, '0'))
      .join(':');

    const dateFormatted = [date.getDate(), date.getMonth() + 1, date.getFullYear()]
      .map(n => n.toString().padStart(2, '0'))
      .join('-');

    return `${dateFormatted} ${time}`;
  }

  private getCallerLocation(omitFunction: Function): string {
    const obj: { stack?: string } = {};
    if (Error.captureStackTrace) {
      Error.captureStackTrace(obj, omitFunction);
    } else {
      obj.stack = new Error().stack;
    }

    const stackLines = obj.stack?.split('\n') ?? [];
    const callerLine = stackLines[1] ?? '';
    const match = callerLine.match(/\((.*)\)/) || callerLine.match(/at (.*)/);
    return match ? match[1].trim() : 'Desconhecido';
  }

  private ensureDirectory(): void {
    if (!this.path) return;
    const dir = dirname(this.path);
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
  }

  public write(content: string, overwrite: boolean = false): void {
    if (!this.path) return;
    try {
      this.ensureDirectory();
      if (overwrite) {
        writeFileSync(this.path, `${content}\n`);
      } else {
        appendFileSync(this.path, `${content}\n`);
      }
    } catch (err) {
      console.error('Falha ao escrever no arquivo de log:', err);
    }
  }

  public clear(): void {
    if (!this.path) return;
    try {
      this.ensureDirectory();
      writeFileSync(this.path, '');
    } catch (err) {
      console.error('Falha ao limpar o arquivo de log:', err);
    }
  }

  public readContent(): string {
    if (!this.path || !existsSync(this.path)) return 'Log de erros vazia.';
    try {
      const data = readFileSync(this.path, 'utf-8');
      return data.trim().length > 0 ? data : 'Log de erros vazia.';
    } catch (err) {
      return `Falha ao ler arquivo de log: ${err}`;
    }
  }

  public log(type: LogType, options: LogOptions = {}) {
    const { message = '', includePath = false, error } = options;
    const timestamp = this.formatDate();

    let location = '';
    if (includePath) {
      if (error instanceof Error && error.stack) {
        const firstLine = error.stack.split('\n')[1] ?? '';
        const match = firstLine.match(/\((.*)\)/) || firstLine.match(/at (.*)/);
        location = match ? match[1].trim() : 'Desconhecido';
      } else {
        location = this.getCallerLocation(this.log);
      }
    }

    const pathSuffixConsole = location ? ` ${gray(`(${location})`)}` : '';
    const pathSuffixFile = location ? ` (${location})` : '';

    switch (type) {
      case LogType.INFO: {
        console.log(`${gray(`[${timestamp}]`)} ${blue(bold('[INFO]'))} ${message}${pathSuffixConsole}`);
        this.write(`[${timestamp}] [INFO] ${message}${pathSuffixFile}`);
        break;
      }
      case LogType.SUCCESS: {
        console.log(`${gray(`[${timestamp}]`)} ${green(bold('[SUCCESS]'))} ${message}${pathSuffixConsole}`);
        this.write(`[${timestamp}] [SUCCESS] ${message}${pathSuffixFile}`);
        break;
      }
      case LogType.WARN: {
        console.warn(`${gray(`[${timestamp}]`)} ${yellow(bold('[WARN]'))} ${message}${pathSuffixConsole}`);
        this.write(`[${timestamp}] [WARN] ${message}${pathSuffixFile}`);
        break;
      }
      case LogType.UPDATE: {
        console.log(`${gray(`[${timestamp}]`)} ${cyan(bold('[UPDATE]'))} ${message}${pathSuffixConsole}`);
        this.write(`[${timestamp}] [UPDATE] ${message}${pathSuffixFile}`);
        break;
      }
      case LogType.ERROR: {
        console.error(`${gray(`[${timestamp}]`)} ${red(bold('[ERROR]'))} ${message}${pathSuffixConsole}`, error ?? '');
        this.write(`[${timestamp}] [ERROR] ${message}${pathSuffixFile} ${error ? (error instanceof Error ? error.stack : JSON.stringify(error)) : ''}`);
        break;
      }
      case LogType.FATAL: {
        this.fatal(message, error);
        break;
      }
    }
  }

  public info(message: string, includePath: boolean = false) {
    this.log(LogType.INFO, { message, includePath });
  }

  public success(message: string, includePath: boolean = false) {
    this.log(LogType.SUCCESS, { message, includePath });
  }

  public warn(message: string, includePath: boolean = false) {
    this.log(LogType.WARN, { message, includePath });
  }

  public update(message: string, includePath: boolean = false) {
    this.log(LogType.UPDATE, { message, includePath });
  }

  public error(message: string, error?: unknown, includePath: boolean = true) {
    this.log(LogType.ERROR, { message, error, includePath });
  }

  public fatal(message: string, error?: unknown): never {
    const timestamp = this.formatDate();

    let location = '';
    let detailedStack = '';

    if (error instanceof Error && error.stack) {
      detailedStack = error.stack;
      const firstLine = error.stack.split('\n')[1] ?? '';
      const match = firstLine.match(/\((.*)\)/) || firstLine.match(/at (.*)/);
      location = match ? match[1].trim() : 'Desconhecido';
    } else {
      location = this.getCallerLocation(this.fatal);
    }

    console.error(`\n${gray(`[${timestamp}]`)} ${red(bold('✖ [FATAL ERROR]'))} ${message}`);
    console.error(`${yellow('📍 Local:')} ${gray(location)}`);
    if (detailedStack) {
      console.error(`${gray(detailedStack)}`);
    }

    const logText = `[${timestamp}] [FATAL] ${message} | Location: ${location}${detailedStack ? `\n${detailedStack}` : ''}`;
    this.write(logText);

    process.exit(1);
  }
}

export const defaultLogger = new Logger();

export const formatDate = (date: Date = new Date()) => defaultLogger.formatDate(date);
export const logContent = async () => defaultLogger.readContent();
export const writeOnLog = async (result: string) => {
  if (result === 'iniciando...') {
    defaultLogger.clear();
    console.log(green('Started Log!'));
    return;
  }
  defaultLogger.write(`[${defaultLogger.formatDate()}] ${result}`);
  console.log(cyan('Updated!'));
};
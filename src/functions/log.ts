import { appendFileSync, readFileSync, writeFileSync } from 'fs'

const formatDate = (date: Date): string => {
  const a = [date.getHours(), date.getMinutes(), date.getSeconds()]
    .map(function (n) { return n.toString().padStart(2, '0'); })
    .join(':');

  const b = [date.getDay(), date.getMonth() + 1, date.getFullYear()]
    .map(function (n) { return n.toString().padStart(2, '0'); })
    .join('-');

  return `${b}` + ' ' + `${a}`;
};

const writeOnLog = async (result: string) => {

  if (result == 'iniciando...') {
    writeFileSync('./src/logs/errorLog.log', '');
    return console.log('Started Log!')
  }

  const hour = formatDate(new Date())

  appendFileSync('./src/logs/errorLog.log', `\n${hour} ${result}`)

  return console.log('Updated!');
}

const logContent = async (): Promise<string | Buffer> => {

  const data = readFileSync('./src/logs/errorLog.log')

  if (data.length <= 0) return 'Log de erros vazia.'

  return data;
}

export { formatDate, writeOnLog, logContent }
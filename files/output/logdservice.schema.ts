import { z } from 'zod';

const LogType = z.enum([
  'UNSPECIFIED',
  'BUCK_LOG',
  'BUCK_MACHINE_LOG',
  'CHROME_TRACE_LOG',
  'SIMPLE_CONSOLE_LOG',
  'CRITICAL_PATH_LOG',
  'RULE_KEY_LOG',
]);
const CreateLogRequest = z.object({
  logFilePath: z.string(),
  logType: LogType,
});
const CreateLogResponse = z.object({
  logId: z.number().int(),
});
const LogMessage = z.object({
  logId: z.number().int(),
  logMessage: z.string(),
});

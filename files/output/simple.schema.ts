import { z } from 'zod';

const Gender = z.enum(['UNKNOWN', 'MALE', 'FEMALE']);
const Person = z.object({
  name: z.string(),
  age: z.number().int(),
  hobbies: z.string(),
  MALE: Gender,
});

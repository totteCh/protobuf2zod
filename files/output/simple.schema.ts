import { z } from 'zod';

const Person = z.object({
  name: z.string(),
  age: z.number().int(),
  hobbies: z.string(),
  MALE: Gender,
});
const Gender = z.enum(['UNKNOWN', 'MALE', 'FEMALE']);

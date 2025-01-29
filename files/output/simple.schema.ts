import { z } from 'zod';

const Gender = z.enum(['UNSPECIFIED', 'MALE', 'FEMALE']);
const Person = z.object({
  name: z.string(),
  age: z.number().int(),
  hobbies: z.string().array(),
  gender: Gender,
});

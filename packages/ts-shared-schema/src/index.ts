import { z } from 'zod'

export const tenantSchema = z.object({
  id: z.string(),
  slug: z.string(),
  displayName: z.string()
})

export type Tenant = z.infer<typeof tenantSchema>


import { queryOptions } from '@tanstack/react-query'
import { getCurrentUser } from './api'

export const currentUserQuery = queryOptions({
  queryKey: ['currentUser'],
  queryFn: getCurrentUser,
  retry: false,
})

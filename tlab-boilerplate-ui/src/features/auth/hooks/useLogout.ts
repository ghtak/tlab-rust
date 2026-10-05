import { useMutation, useQueryClient } from '@tanstack/react-query'
import { logout } from '../services/api'

export function useLogout() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: logout,
    onSuccess: () => {
      queryClient.clear()
    },
  })
}

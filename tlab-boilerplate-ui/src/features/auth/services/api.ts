import { HTTPError } from 'ky'
import { api } from '../../../api'
import type { CurrentUser } from '../types'

export async function getCurrentUser(): Promise<CurrentUser | null> {
  try {
    const response = await api.get('auth/me').json<{ data: CurrentUser }>()
    return response.data
  } catch (error) {
    if (error instanceof HTTPError && error.response.status === 401) return null
    throw error
  }
}

export async function login(email: string, password: string) {
  await api.post('auth/login', { json: { email, password } })
}

export async function logout() {
  await api.post('auth/logout')
}

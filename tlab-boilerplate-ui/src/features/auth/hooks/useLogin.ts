import { useMutation, useQueryClient } from "@tanstack/react-query";
import { login } from "../services/api";
import { currentUserQuery } from "../services/queries";

export function useLogin() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({ email, password }: { email: string; password: string }) =>
			login(email, password),
		onSuccess: async () => {
			await queryClient.invalidateQueries({
				queryKey: currentUserQuery.queryKey,
			});
		},
	});
}

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { login } from "../services/api";
import {
	authVerificationQuery,
	currentUserQuery,
} from "../services/queries";

export function useLogin() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({ email, password }: { email: string; password: string }) =>
			login(email, password),
		onSuccess: async () => {
			queryClient.setQueryData(authVerificationQuery.queryKey, true);
			queryClient.removeQueries({ queryKey: currentUserQuery.queryKey });
			await queryClient.fetchQuery(currentUserQuery);
		},
	});
}

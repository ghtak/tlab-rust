import { useQuery } from "@tanstack/react-query";
import { currentUserQuery } from "../services/queries";

export function useCurrentUser() {
	const { data, isLoading, error } = useQuery(currentUserQuery);

	return {
		user: data ?? null,
		permissions: data?.permissions ?? [],
		isLoading,
		error,
	};
}

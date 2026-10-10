import { queryOptions } from "@tanstack/react-query";
import { getCurrentUser, verifyAuth } from "./api";

export const authVerificationQuery = queryOptions({
	queryKey: ["authVerification"],
	queryFn: verifyAuth,
	retry: false,
});

export const currentUserQuery = queryOptions({
	queryKey: ["currentUser"],
	queryFn: getCurrentUser,
	staleTime: Infinity,
	retry: false,
});

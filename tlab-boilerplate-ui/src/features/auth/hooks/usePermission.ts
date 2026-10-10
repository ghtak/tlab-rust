import { hasAllPermissions, hasAnyPermission, hasPermission } from "../access";
import { useCurrentUser } from "./useCurrentUser";

export function usePermission() {
	const { user, isLoading } = useCurrentUser();
	const ready = !isLoading && user !== null;

	return {
		hasPermission: (permission: string) =>
			ready && hasPermission(user, permission),
		hasAnyPermission: (permissions: string[]) =>
			ready && hasAnyPermission(user, permissions),
		hasAllPermissions: (permissions: string[]) =>
			ready && hasAllPermissions(user, permissions),
		isLoading,
	};
}

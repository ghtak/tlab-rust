import { queryOptions } from "@tanstack/react-query";
import type { PermissionListParams, RoleListParams } from "../types";
import { getPermissions, getRolePermissions, getRoles } from "./api";

export function permissionsQuery(params: PermissionListParams) {
	return queryOptions({
		queryKey: ["permissions", params],
		queryFn: () => getPermissions(params),
		retry: false,
	});
}

export function rolesQuery(params: RoleListParams) {
	return queryOptions({
		queryKey: ["roles", params],
		queryFn: () => getRoles(params),
		retry: false,
	});
}

export function rolePermissionsQuery(roleId: number) {
	return queryOptions({
		queryKey: ["rolePermissions", roleId],
		queryFn: () => getRolePermissions(roleId),
		retry: false,
	});
}

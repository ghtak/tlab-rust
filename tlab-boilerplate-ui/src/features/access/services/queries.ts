import { queryOptions } from "@tanstack/react-query";
import type {
	PermissionListParams,
	RoleListParams,
	UserListParams,
} from "../types";
import {
	getPermissions,
	getRolePermissions,
	getRoles,
	getUserRoles,
	getUsers,
} from "./api";

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

export function usersQuery(params: UserListParams) {
	return queryOptions({
		queryKey: ["users", params],
		queryFn: () => getUsers(params),
		retry: false,
	});
}

export function userRolesQuery(userId: number) {
	return queryOptions({
		queryKey: ["userRoles", userId],
		queryFn: () => getUserRoles(userId),
		retry: false,
	});
}

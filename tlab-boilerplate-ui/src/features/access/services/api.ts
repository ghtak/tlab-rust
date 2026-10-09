import { api } from "../../../api";
import type {
	ChangeRolePermissionsInput,
	ChangeUserRolesInput,
	CreatePermissionInput,
	CreateRoleInput,
	Permission,
	PermissionList,
	PermissionListParams,
	RoleList,
	RoleListParams,
	RolePermissions,
	RoleWithPermissionCount,
	UserList,
	UserListParams,
	UserRoles,
} from "../types";

export async function createPermission(
	input: CreatePermissionInput,
): Promise<Permission> {
	const response = await api
		.post("auth/permissions", { json: input })
		.json<{ data: Permission }>();
	return response.data;
}

export async function deletePermission(id: number): Promise<void> {
	await api.delete(`auth/permissions/${id}`);
}

export async function getPermissions({
	code,
	page,
	pageSize,
}: PermissionListParams): Promise<PermissionList> {
	const response = await api
		.get("auth/permissions", {
			searchParams: {
				...(code ? { code } : {}),
				page: String(page),
				page_size: String(pageSize),
			},
		})
		.json<{ data: PermissionList }>();
	return response.data;
}

export async function createRole(
	input: CreateRoleInput,
): Promise<RoleWithPermissionCount> {
	const response = await api
		.post("auth/roles", { json: input })
		.json<{ data: RoleWithPermissionCount }>();
	return response.data;
}

export async function deleteRole(id: number): Promise<void> {
	await api.delete(`auth/roles/${id}`);
}

export async function getRoles({
	code,
	page,
	pageSize,
}: RoleListParams): Promise<RoleList> {
	const response = await api
		.get("auth/roles", {
			searchParams: {
				...(code ? { code } : {}),
				page: String(page),
				page_size: String(pageSize),
			},
		})
		.json<{ data: RoleList }>();
	return response.data;
}

export async function getRolePermissions(
	roleId: number,
): Promise<RolePermissions> {
	const response = await api
		.get(`auth/roles/${roleId}/permissions`)
		.json<{ data: RolePermissions }>();
	return response.data;
}

export async function changeRolePermissions({
	roleId,
	add_ids,
	remove_ids,
}: ChangeRolePermissionsInput): Promise<void> {
	await api.patch(`auth/roles/${roleId}/permissions`, {
		json: { add_ids, remove_ids },
	});
}

export async function getUsers({
	q,
	status,
	page,
	pageSize,
}: UserListParams): Promise<UserList> {
	const response = await api
		.get("auth/users", {
			searchParams: {
				...(q ? { q } : {}),
				...(status ? { status } : {}),
				page: String(page),
				page_size: String(pageSize),
			},
		})
		.json<{ data: UserList }>();
	return response.data;
}

export async function getUserRoles(userId: number): Promise<UserRoles> {
	const response = await api
		.get(`auth/users/${userId}/roles`)
		.json<{ data: UserRoles }>();
	return response.data;
}

export async function changeUserRoles({
	userId,
	add_ids,
	remove_ids,
}: ChangeUserRolesInput): Promise<void> {
	await api.patch(`auth/users/${userId}/roles`, {
		json: { add_ids, remove_ids },
	});
}

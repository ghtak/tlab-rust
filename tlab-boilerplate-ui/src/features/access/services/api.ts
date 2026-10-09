import { api } from "../../../api";
import type {
	CreatePermissionInput,
	CreateRoleInput,
	Permission,
	PermissionList,
	PermissionListParams,
	Role,
	RoleList,
	RoleListParams,
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

export async function createRole(input: CreateRoleInput): Promise<Role> {
	const response = await api
		.post("auth/roles", { json: input })
		.json<{ data: Role }>();
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

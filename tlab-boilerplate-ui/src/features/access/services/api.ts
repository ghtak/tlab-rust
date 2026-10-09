import { api } from "../../../api";
import type {
	CreatePermissionInput,
	Permission,
	PermissionList,
	PermissionListParams,
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

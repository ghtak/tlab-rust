export type Permission = {
	id: number;
	code: string;
	description: string | null;
};

export type PermissionListParams = {
	code: string;
	page: number;
	pageSize: number;
};

export type PermissionList = {
	items: Permission[];
	total: number;
	page: number;
	page_size: number;
};

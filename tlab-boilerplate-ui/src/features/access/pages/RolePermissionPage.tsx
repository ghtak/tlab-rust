import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { HTTPError } from "ky";
import { useEffect, useState } from "react";
import { Button } from "../../../components/ui/button";
import { Card, CardHeader, CardTitle } from "../../../components/ui/card";
import { Checkbox } from "../../../components/ui/checkbox";
import { Input } from "../../../components/ui/input";
import {
	Pagination,
	PaginationContent,
	PaginationItem,
	PaginationLink,
	PaginationNext,
	PaginationPrevious,
} from "../../../components/ui/pagination";
import { changeRolePermissions } from "../services/api";
import { permissionsQuery, rolePermissionsQuery } from "../services/queries";
import type { Permission, RolePermissions } from "../types";

const PAGE_SIZE = 20;

export function RolePermissionPage({ roleId }: { roleId: number }) {
	const validRoleId = Number.isSafeInteger(roleId) && roleId > 0;
	const { data, error, isPending } = useQuery({
		...rolePermissionsQuery(roleId),
		enabled: validRoleId,
	});

	if (!validRoleId) {
		return <p role="alert">잘못된 롤 주소입니다.</p>;
	}
	if (isPending) return <p>롤의 퍼미션을 불러오는 중입니다.</p>;
	if (error) {
		return (
			<p role="alert">
				{error instanceof HTTPError && error.response.status === 404
					? "롤을 찾을 수 없습니다."
					: "롤의 퍼미션을 불러오지 못했습니다."}
			</p>
		);
	}
	return <RolePermissionEditor key={roleId} initial={data} />;
}

function RolePermissionEditor({ initial }: { initial: RolePermissions }) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const [inputCode, setInputCode] = useState("");
	const [code, setCode] = useState("");
	const [page, setPage] = useState(1);
	const [filter, setFilter] = useState<"all" | "linked">("all");
	const [selected, setSelected] = useState(
		() => new Set(initial.permissions.map((permission) => permission.id)),
	);
	const [known, setKnown] = useState(
		() =>
			new Map(
				initial.permissions.map((permission) => [permission.id, permission]),
			),
	);
	const catalog = useQuery({
		...permissionsQuery({ code, page, pageSize: PAGE_SIZE }),
		enabled: filter === "all",
	});
	useEffect(() => {
		if (!catalog.data) return;
		setKnown((current) => {
			const next = new Map(current);
			for (const permission of catalog.data.items)
				next.set(permission.id, permission);
			return next;
		});
	}, [catalog.data]);
	const saved = new Set(initial.permissions.map((permission) => permission.id));
	const addIds = [...selected].filter((id) => !saved.has(id));
	const removeIds = [...saved].filter((id) => !selected.has(id));
	const save = useMutation({
		mutationFn: changeRolePermissions,
		onSuccess: async () => {
			await Promise.all([
				queryClient.invalidateQueries({ queryKey: ["roles"] }),
				queryClient.invalidateQueries({
					queryKey: ["rolePermissions", initial.role.id],
				}),
			]);
			void navigate({ to: "/roles" });
		},
	});
	const linkedItems = [...selected]
		.map((id) => known.get(id))
		.filter((permission): permission is Permission => Boolean(permission))
		.filter((permission) =>
			permission.code.toLowerCase().includes(code.toLowerCase()),
		)
		.sort((a, b) => a.code.localeCompare(b.code));
	const items = filter === "linked" ? linkedItems : (catalog.data?.items ?? []);
	const totalPages = Math.max(
		1,
		Math.ceil((catalog.data?.total ?? 0) / PAGE_SIZE),
	);

	return (
		<div className="space-y-5">
			<div>
				<p className="mb-2 text-sm text-slate-500">
					접근 관리 /{" "}
					<Link to="/roles" className="hover:underline">
						롤
					</Link>{" "}
					/ {initial.role.code} / 퍼미션 관리
				</p>
				<h1 className="text-3xl font-semibold tracking-tight">
					{initial.role.code} 퍼미션 관리
				</h1>
				<p className="mt-1 text-sm text-slate-500">
					현재 {selected.size}개 퍼미션이 연결됩니다.
				</p>
			</div>
			<Card className="gap-0 bg-white py-0 shadow-sm">
				<CardHeader className="flex flex-col gap-3 border-b border-slate-100 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-6">
					<div className="flex items-center gap-3">
						<CardTitle>퍼미션 선택</CardTitle>
						<Button
							variant={filter === "all" ? "secondary" : "ghost"}
							size="sm"
							aria-pressed={filter === "all"}
							onClick={() => setFilter("all")}
						>
							전체
						</Button>
						<Button
							variant={filter === "linked" ? "secondary" : "ghost"}
							size="sm"
							aria-pressed={filter === "linked"}
							onClick={() => setFilter("linked")}
						>
							연결됨
						</Button>
					</div>
					<form
						className="flex w-full gap-2 sm:w-80"
						onSubmit={(event) => {
							event.preventDefault();
							setCode(inputCode.trim());
							setPage(1);
						}}
					>
						<label htmlFor="role-permission-code" className="sr-only">
							퍼미션 코드 검색
						</label>
						<Input
							id="role-permission-code"
							type="search"
							placeholder="코드 검색"
							value={inputCode}
							onChange={(event) => setInputCode(event.target.value)}
						/>
						<Button type="submit">검색</Button>
					</form>
				</CardHeader>
				{filter === "all" && catalog.isPending ? (
					<output className="block px-5 py-12 text-center text-sm text-slate-500">
						퍼미션을 불러오는 중입니다.
					</output>
				) : filter === "all" && catalog.error ? (
					<p
						role="alert"
						className="px-5 py-12 text-center text-sm text-destructive"
					>
						퍼미션 목록을 불러오지 못했습니다.
					</p>
				) : items.length === 0 ? (
					<p className="px-5 py-12 text-center text-sm text-slate-500">
						표시할 퍼미션이 없습니다.
					</p>
				) : (
					<div className="divide-y divide-slate-100">
						{items.map((permission) => {
							const protectedPermission =
								initial.role.code === "admin" &&
								permission.code === "access:manage" &&
								saved.has(permission.id);
							return (
								<div
									key={permission.id}
									className="flex items-center gap-3 px-5 py-3 sm:px-6"
								>
									<Checkbox
										id={`role-permission-${permission.id}`}
										checked={selected.has(permission.id)}
										disabled={protectedPermission || save.isPending}
										onCheckedChange={(checked) => {
											setSelected((current) => {
												const next = new Set(current);
												if (checked) next.add(permission.id);
												else next.delete(permission.id);
												return next;
											});
										}}
									/>
									<label
										htmlFor={`role-permission-${permission.id}`}
										className="min-w-0 flex-1 cursor-pointer"
									>
										<span className="block font-medium">{permission.code}</span>
										<span className="block text-sm text-slate-500">
											{protectedPermission
												? "기본 관리자 롤에서 해제할 수 없습니다."
												: (permission.description ?? "설명 없음")}
										</span>
									</label>
								</div>
							);
						})}
					</div>
				)}
				{filter === "all" && (
					<div className="border-t border-slate-100 px-5 py-4 sm:px-6">
						<Pagination>
							<PaginationContent>
								<PaginationItem>
									<PaginationPrevious
										href="#"
										text="이전"
										aria-label="이전 페이지"
										aria-disabled={page <= 1 || catalog.isPending}
										tabIndex={page <= 1 || catalog.isPending ? -1 : undefined}
										onClick={(event) => {
											event.preventDefault();
											if (page > 1 && !catalog.isPending) setPage(page - 1);
										}}
									/>
								</PaginationItem>
								<PaginationItem>
									<PaginationLink
										href="#"
										isActive
										aria-label={`${page} 페이지`}
										onClick={(event) => event.preventDefault()}
									>
										{page}
									</PaginationLink>
								</PaginationItem>
								<PaginationItem>
									<PaginationNext
										href="#"
										text="다음"
										aria-label="다음 페이지"
										aria-disabled={page >= totalPages || catalog.isPending}
										tabIndex={
											page >= totalPages || catalog.isPending ? -1 : undefined
										}
										onClick={(event) => {
											event.preventDefault();
											if (page < totalPages && !catalog.isPending)
												setPage(page + 1);
										}}
									/>
								</PaginationItem>
							</PaginationContent>
						</Pagination>
					</div>
				)}
			</Card>
			<div className="flex flex-wrap items-center justify-between gap-3">
				<p className="text-sm text-slate-600">
					변경: 추가 {addIds.length}개 · 해제 {removeIds.length}개
				</p>
				<div className="flex gap-2">
					<Button
						variant="outline"
						disabled={save.isPending}
						onClick={() => void navigate({ to: "/roles" })}
					>
						취소
					</Button>
					<Button
						disabled={
							save.isPending || (addIds.length === 0 && removeIds.length === 0)
						}
						onClick={() =>
							save.mutate({
								roleId: initial.role.id,
								permission_ids: [...selected],
							})
						}
					>
						{save.isPending ? "저장 중..." : "저장"}
					</Button>
				</div>
			</div>
			{save.isError && (
				<p role="alert" className="text-sm text-destructive">
					{save.error instanceof HTTPError && save.error.response.status === 400
						? "선택한 퍼미션과 변경 내용을 확인해 주세요."
						: "변경사항을 저장하지 못했습니다. 다시 시도해 주세요."}
				</p>
			)}
		</div>
	);
}

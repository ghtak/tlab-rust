import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { HTTPError } from "ky";
import { KeyRoundIcon, MoreHorizontalIcon, Trash2Icon } from "lucide-react";
import { useState } from "react";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "../../../components/ui/alert-dialog";
import { Button } from "../../../components/ui/button";
import { Card, CardHeader, CardTitle } from "../../../components/ui/card";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "../../../components/ui/dropdown-menu";
import { Input } from "../../../components/ui/input";
import {
	Pagination,
	PaginationContent,
	PaginationItem,
	PaginationLink,
	PaginationNext,
	PaginationPrevious,
} from "../../../components/ui/pagination";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "../../../components/ui/table";
import { CreateRoleDialog } from "../components/CreateRoleDialog";
import { deleteRole } from "../services/api";
import { rolesQuery } from "../services/queries";
import type { Role } from "../types";

const PAGE_SIZE = 20;

export function RolePage() {
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const [inputCode, setInputCode] = useState("");
	const [code, setCode] = useState("");
	const [page, setPage] = useState(1);
	const [roleToDelete, setRoleToDelete] = useState<Role | null>(null);
	const { data, error, isPending } = useQuery(
		rolesQuery({ code, page, pageSize: PAGE_SIZE }),
	);
	const deletion = useMutation({
		mutationFn: deleteRole,
		onSuccess: async () => {
			setRoleToDelete(null);
			if (data?.items.length === 1 && page > 1) setPage(page - 1);
			await queryClient.invalidateQueries({ queryKey: ["roles"] });
		},
	});
	const totalPages = Math.max(1, Math.ceil((data?.total ?? 0) / PAGE_SIZE));
	const firstVisiblePage = Math.max(1, Math.min(page - 2, totalPages - 4));
	const visiblePages = Array.from(
		{ length: Math.min(5, totalPages - firstVisiblePage + 1) },
		(_, index) => firstVisiblePage + index,
	);

	return (
		<div className="space-y-5">
			<div className="flex flex-wrap items-end justify-between gap-4">
				<div>
					<p className="mb-2 text-sm text-slate-500">
						접근 관리 / <span className="text-slate-900">롤</span>
					</p>
					<h1 className="text-3xl font-semibold tracking-tight">롤</h1>
					<p className="mt-1 text-sm text-slate-500">
						서비스에서 사용하는 롤을 조회합니다.
					</p>
				</div>
				<CreateRoleDialog />
			</div>

			<Card className="gap-0 bg-white py-0 shadow-sm">
				<CardHeader className="flex flex-col gap-3 border-b border-slate-100 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-6">
					<div className="flex items-center gap-3">
						<CardTitle>목록</CardTitle>
						<span className="text-sm text-slate-500">
							{data ? `${data.total}개 항목` : "항목 조회 중"}
						</span>
					</div>
					<form
						className="flex w-full gap-2 sm:w-80"
						onSubmit={(event) => {
							event.preventDefault();
							setCode(inputCode.trim());
							setPage(1);
						}}
					>
						<label htmlFor="role-code" className="sr-only">
							롤 코드 검색
						</label>
						<Input
							id="role-code"
							type="search"
							placeholder="코드 검색"
							value={inputCode}
							onChange={(event) => setInputCode(event.target.value)}
						/>
						<Button type="submit">검색</Button>
					</form>
				</CardHeader>
				{isPending ? (
					<output className="block px-5 py-12 text-center text-sm text-slate-500">
						롤을 불러오는 중입니다.
					</output>
				) : error ? (
					<p
						role="alert"
						className="px-5 py-12 text-center text-sm text-destructive"
					>
						{error instanceof HTTPError && error.response.status === 403
							? "롤 목록을 볼 권한이 없습니다."
							: "롤 목록을 불러오지 못했습니다."}
					</p>
				) : data.items.length === 0 ? (
					<p className="px-5 py-12 text-center text-sm text-slate-500">
						{code ? "검색 결과가 없습니다." : "등록된 롤이 없습니다."}
					</p>
				) : (
					<Table className="min-w-[620px]">
						<TableHeader className="bg-slate-50 text-xs text-slate-500">
							<TableRow>
								<TableHead className="px-5 sm:px-6">코드</TableHead>
								<TableHead className="px-5 sm:px-6">설명</TableHead>
								<TableHead className="px-5 sm:px-6">퍼미션</TableHead>
								<TableHead className="w-16 px-5 text-right sm:px-6">
									작업
								</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{data.items.map((role) => (
								<TableRow key={role.id}>
									<TableCell className="px-5 py-4 font-medium sm:px-6">
										{role.code}
									</TableCell>
									<TableCell className="px-5 py-4 text-slate-600 sm:px-6">
										{role.description ?? "—"}
									</TableCell>
									<TableCell className="px-5 py-4 text-slate-600 sm:px-6">
										{role.permission_count}개
									</TableCell>
									<TableCell className="px-5 py-4 text-right sm:px-6">
										<DropdownMenu>
											<DropdownMenuTrigger
												render={<Button variant="ghost" size="icon-sm" />}
												aria-label={`${role.code} 작업`}
											>
												<MoreHorizontalIcon />
											</DropdownMenuTrigger>
											<DropdownMenuContent align="end">
												<DropdownMenuItem
													onClick={() =>
														void navigate({
															to: "/roles/$roleId/permissions",
															params: { roleId: String(role.id) },
														})
													}
												>
													<KeyRoundIcon /> 퍼미션 관리
												</DropdownMenuItem>
												<DropdownMenuItem
													variant="destructive"
													disabled={role.code === "admin"}
													onClick={() => {
														deletion.reset();
														setRoleToDelete(role);
													}}
												>
													<Trash2Icon />{" "}
													{role.code === "admin"
														? "기본 관리자 롤은 삭제할 수 없습니다"
														: "삭제"}
												</DropdownMenuItem>
											</DropdownMenuContent>
										</DropdownMenu>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				)}
				<div className="border-t border-slate-100 px-5 py-4 sm:px-6">
					<Pagination>
						<PaginationContent>
							<PaginationItem>
								<PaginationPrevious
									href="#"
									text="이전"
									aria-label="이전 페이지"
									aria-disabled={page <= 1 || isPending}
									tabIndex={page <= 1 || isPending ? -1 : undefined}
									onClick={(event) => {
										event.preventDefault();
										if (page > 1 && !isPending) setPage(page - 1);
									}}
								/>
							</PaginationItem>
							{visiblePages.map((visiblePage) => (
								<PaginationItem key={visiblePage}>
									<PaginationLink
										href="#"
										isActive={visiblePage === page}
										aria-label={`${visiblePage} 페이지`}
										aria-disabled={isPending}
										tabIndex={isPending ? -1 : undefined}
										onClick={(event) => {
											event.preventDefault();
											if (!isPending) setPage(visiblePage);
										}}
									>
										{visiblePage}
									</PaginationLink>
								</PaginationItem>
							))}
							<PaginationItem>
								<PaginationNext
									href="#"
									text="다음"
									aria-label="다음 페이지"
									aria-disabled={!data || page >= totalPages || isPending}
									tabIndex={
										!data || page >= totalPages || isPending ? -1 : undefined
									}
									onClick={(event) => {
										event.preventDefault();
										if (data && page < totalPages && !isPending)
											setPage(page + 1);
									}}
								/>
							</PaginationItem>
						</PaginationContent>
					</Pagination>
				</div>
			</Card>
			<AlertDialog
				open={roleToDelete !== null}
				onOpenChange={(open) => {
					if (!open && !deletion.isPending) setRoleToDelete(null);
				}}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>롤을 삭제할까요?</AlertDialogTitle>
						<AlertDialogDescription>
							{roleToDelete?.code} 롤을 삭제하면 연결된 퍼미션도 해제됩니다. 이
							작업은 되돌릴 수 없습니다.
						</AlertDialogDescription>
					</AlertDialogHeader>
					{deletion.isError && (
						<p role="alert" className="text-sm text-destructive">
							{deletion.error instanceof HTTPError &&
							deletion.error.response.status === 409
								? "사용자에게 할당된 롤은 삭제할 수 없습니다."
								: "롤을 삭제하지 못했습니다. 다시 시도해 주세요."}
						</p>
					)}
					<AlertDialogFooter>
						<AlertDialogCancel disabled={deletion.isPending} autoFocus>
							취소
						</AlertDialogCancel>
						<AlertDialogAction
							variant="destructive"
							disabled={deletion.isPending}
							onClick={() => {
								if (roleToDelete) deletion.mutate(roleToDelete.id);
							}}
						>
							{deletion.isPending ? "삭제 중..." : "삭제"}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</div>
	);
}

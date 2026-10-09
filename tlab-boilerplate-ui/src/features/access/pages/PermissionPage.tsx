import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { HTTPError } from "ky";
import { MoreHorizontalIcon, Trash2Icon } from "lucide-react";
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
import { CreatePermissionDialog } from "../components/CreatePermissionDialog";
import { deletePermission } from "../services/api";
import { permissionsQuery } from "../services/queries";
import type { Permission } from "../types";

const PAGE_SIZE = 20;

export function PermissionPage() {
	const queryClient = useQueryClient();
	const [inputCode, setInputCode] = useState("");
	const [code, setCode] = useState("");
	const [page, setPage] = useState(1);
	const [permissionToDelete, setPermissionToDelete] =
		useState<Permission | null>(null);
	const { data, error, isPending } = useQuery(
		permissionsQuery({ code, page, pageSize: PAGE_SIZE }),
	);
	const deletion = useMutation({
		mutationFn: deletePermission,
		onSuccess: async () => {
			setPermissionToDelete(null);
			if (data?.items.length === 1 && page > 1) setPage(page - 1);
			await queryClient.invalidateQueries({ queryKey: ["permissions"] });
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
						접근 관리 / <span className="text-slate-900">퍼미션</span>
					</p>
					<h1 className="text-3xl font-semibold tracking-tight">퍼미션</h1>
					<p className="mt-1 text-sm text-slate-500">
						서비스에서 사용하는 퍼미션을 조회합니다.
					</p>
				</div>
				<CreatePermissionDialog />
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
						<label htmlFor="permission-code" className="sr-only">
							퍼미션 코드 검색
						</label>
						<Input
							id="permission-code"
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
						퍼미션을 불러오는 중입니다.
					</output>
				) : error ? (
					<p
						role="alert"
						className="px-5 py-12 text-center text-sm text-destructive"
					>
						{error instanceof HTTPError && error.response.status === 403
							? "퍼미션 목록을 볼 권한이 없습니다."
							: "퍼미션 목록을 불러오지 못했습니다."}
					</p>
				) : data.items.length === 0 ? (
					<p className="px-5 py-12 text-center text-sm text-slate-500">
						{code ? "검색 결과가 없습니다." : "등록된 퍼미션이 없습니다."}
					</p>
				) : (
					<Table className="min-w-[620px]">
						<TableHeader className="bg-slate-50 text-xs text-slate-500">
							<TableRow>
								<TableHead className="px-5 sm:px-6">코드</TableHead>
								<TableHead className="px-5 sm:px-6">설명</TableHead>
								<TableHead className="w-16 px-5 text-right sm:px-6">
									작업
								</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{data.items.map((permission) => (
								<TableRow key={permission.id}>
									<TableCell className="px-5 py-4 font-medium sm:px-6">
										{permission.code}
									</TableCell>
									<TableCell className="px-5 py-4 text-slate-600 sm:px-6">
										{permission.description ?? "—"}
									</TableCell>
									<TableCell className="px-5 py-4 text-right sm:px-6">
										<DropdownMenu>
											<DropdownMenuTrigger
												render={<Button variant="ghost" size="icon-sm" />}
												aria-label={`${permission.code} 작업`}
											>
												<MoreHorizontalIcon />
											</DropdownMenuTrigger>
											<DropdownMenuContent align="end">
												<DropdownMenuItem
													variant="destructive"
													onClick={() => {
														deletion.reset();
														setPermissionToDelete(permission);
													}}
												>
													<Trash2Icon /> 삭제
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
				open={permissionToDelete !== null}
				onOpenChange={(open) => {
					if (!open && !deletion.isPending) setPermissionToDelete(null);
				}}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>퍼미션을 삭제할까요?</AlertDialogTitle>
						<AlertDialogDescription>
							{permissionToDelete?.code} 퍼미션을 삭제하면 모든 롤에서 연결이
							제거됩니다. 이 작업은 되돌릴 수 없습니다.
						</AlertDialogDescription>
					</AlertDialogHeader>
					{deletion.isError && (
						<p role="alert" className="text-sm text-destructive">
							퍼미션을 삭제하지 못했습니다. 다시 시도해 주세요.
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
								if (permissionToDelete) deletion.mutate(permissionToDelete.id);
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

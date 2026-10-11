import { useMutation, useQueryClient } from "@tanstack/react-query";
import { HTTPError } from "ky";
import { useState } from "react";
import { FormattedMessage, useIntl } from "react-intl";
import { Button } from "../../../components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "../../../components/ui/dialog";
import { Input } from "../../../components/ui/input";
import { Label } from "../../../components/ui/label";
import { Textarea } from "../../../components/ui/textarea";
import { createPermission } from "../services/api";

export function CreatePermissionDialog() {
	const intl = useIntl();
	const queryClient = useQueryClient();
	const [open, setOpen] = useState(false);
	const [code, setCode] = useState("");
	const [description, setDescription] = useState("");
	const [codeError, setCodeError] = useState(false);
	const create = useMutation({
		mutationFn: createPermission,
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["permissions"] });
			setOpen(false);
		},
	});
	const duplicate =
		create.error instanceof HTTPError && create.error.response.status === 409;

	return (
		<Dialog
			open={open}
			onOpenChange={(nextOpen) => {
				if (nextOpen) {
					setCode("");
					setDescription("");
					setCodeError(false);
					create.reset();
				}
				setOpen(nextOpen);
			}}
		>
			<DialogTrigger render={<Button />}>
				<FormattedMessage
					id="access.permission.add"
					defaultMessage="퍼미션 추가"
				/>
			</DialogTrigger>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						<FormattedMessage
							id="access.permission.add"
							defaultMessage="퍼미션 추가"
						/>
					</DialogTitle>
					<DialogDescription>
						<FormattedMessage
							id="access.permission.addDescription"
							defaultMessage="새 퍼미션의 코드와 설명을 입력합니다."
						/>
					</DialogDescription>
				</DialogHeader>
				<form
					className="space-y-4"
					onSubmit={(event) => {
						event.preventDefault();
						const trimmedCode = code.trim();
						if (!trimmedCode) {
							setCodeError(true);
							return;
						}
						create.mutate({
							code: trimmedCode,
							description: description.trim() || null,
						});
					}}
				>
					<div className="space-y-2">
						<Label htmlFor="new-permission-code">
							<FormattedMessage id="common.codeLabel" defaultMessage="코드 *" />
						</Label>
						<Input
							id="new-permission-code"
							disabled={create.isPending}
							maxLength={100}
							placeholder={intl.formatMessage({
								id: "access.permission.codeExample",
								defaultMessage: "예: order:read",
							})}
							value={code}
							aria-invalid={codeError || duplicate}
							aria-describedby={
								codeError || duplicate ? "new-permission-code-error" : undefined
							}
							onChange={(event) => {
								setCode(event.target.value);
								setCodeError(false);
								create.reset();
							}}
						/>
						{(codeError || duplicate) && (
							<p
								id="new-permission-code-error"
								className="text-sm text-destructive"
							>
								{intl.formatMessage(
									codeError
										? {
												id: "common.codeRequired",
												defaultMessage: "코드를 입력해 주세요.",
											}
										: {
												id: "common.codeDuplicate",
												defaultMessage: "이미 등록된 코드입니다.",
											},
								)}
							</p>
						)}
					</div>
					<div className="space-y-2">
						<Label htmlFor="new-permission-description">
							<FormattedMessage id="common.description" defaultMessage="설명" />
						</Label>
						<Textarea
							id="new-permission-description"
							disabled={create.isPending}
							rows={3}
							value={description}
							onChange={(event) => setDescription(event.target.value)}
						/>
					</div>
					{create.isError && !duplicate && (
						<p role="alert" className="text-sm text-destructive">
							<FormattedMessage
								id="access.permission.addFailed"
								defaultMessage="퍼미션을 추가하지 못했습니다. 다시 시도해 주세요."
							/>
						</p>
					)}
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={create.isPending}
							onClick={() => setOpen(false)}
						>
							<FormattedMessage id="common.cancel" defaultMessage="취소" />
						</Button>
						<Button type="submit" disabled={create.isPending}>
							{create.isPending ? (
								<FormattedMessage
									id="common.adding"
									defaultMessage="추가 중..."
								/>
							) : (
								<FormattedMessage id="common.add" defaultMessage="추가" />
							)}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}

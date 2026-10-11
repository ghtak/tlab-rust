# UI 테마와 스타일 작성

새 화면은 기존 Tailwind CSS와 shadcn/ui 컴포넌트를 사용한다. 색상은 화면에서 역할을 나타내는 토큰으로 지정해 라이트·다크 테마에 함께 대응한다. 현재 값과 적용 흐름의 기준은 [`styles.css`](../../src/styles.css), [`ThemeProvider`](../../src/lib/ThemeProvider.tsx), [`ThemeSelect`](../../src/components/ThemeSelect.tsx)다.

## 테마가 적용되는 흐름

`styles.css`의 `@theme inline`은 Tailwind 유틸리티 이름과 CSS 변수를 연결한다.

```css
@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
}

:root {
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
}

.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
}
```

`--color-background`를 등록하면 `bg-background` 같은 Tailwind 클래스를 쓸 수 있다. 이 클래스는 `--background` 값을 참조한다. `<html>`에 `dark` 클래스가 붙으면 `.dark`의 변수값이 적용되므로 컴포넌트의 클래스 이름을 바꿀 필요가 없다. `color-scheme`은 브라우저 기본 입력 요소 등의 밝은·어두운 표시를 지정하며, 앱 색상은 위 CSS 변수가 결정한다.

[`index.html`](../../index.html)은 React가 실행되기 전에 저장된 선택 또는 운영체제 설정을 읽어 `dark` 클래스를 적용한다. 앱이 시작되면 `ThemeProvider`가 `light | dark | system` 선택을 `tlab.theme`에 저장하고 루트 클래스를 갱신한다. `system` 선택 중에는 운영체제 설정 변경도 반영한다. 선택 UI는 로그인 화면, 계정 화면, 공통 [`AppLayout`](../../src/features/app-shell/AppLayout.tsx) 헤더에서 `ThemeSelect`를 재사용한다.

## 새 화면에서 스타일 쓰기

- 배경·글자·테두리에는 `bg-background`, `bg-card`, `text-foreground`, `text-muted-foreground`, `border-border`처럼 역할 기반 클래스를 사용한다. `bg-white`, `text-slate-900`처럼 한 테마에서만 맞는 고정 색상은 피한다.
- 버튼·카드·입력 등은 [`src/components/ui`](../../src/components/ui)의 기존 컴포넌트와 variant를 먼저 확인한다. 페이지마다 같은 외형을 다시 정의하지 않는다.
- 상태에 따라 색 조합이 달라져야 하면 의미에 맞는 기존 토큰을 사용하거나 `dark:` 변형으로 두 테마의 값을 함께 지정한다. 새 공통 색상이 필요할 때만 `styles.css`의 `@theme inline` 연결과 `:root`·`.dark` 값을 추가한다.
- 복잡한 표·셀 선택자, 고정 열 등 한 컴포넌트에만 속하는 규칙은 CSS Module에 둘 수 있다. Module 안에서도 `var(--background)`, `var(--border)` 등 공통 변수를 사용한다. 단순한 간격·배치·색상은 기존 Tailwind 클래스를 사용한다.
- 실행 중 계산되는 차트 너비 같은 단일 값은 `style` 속성으로 전달할 수 있다. 공통 외형은 클래스와 토큰으로 관리한다.

새 화면을 확인할 때는 라이트와 다크 양쪽에서 표면, 글자, 상태 표시, 입력 요소의 대비를 확인한다. 시스템 선택도 운영체제 설정에 따라 전환되는지 확인한다.

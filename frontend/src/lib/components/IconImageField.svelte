<script lang="ts">
	import { ImagePlus, X } from '@lucide/svelte';

	interface Props {
		value?: string;
	}

	const imageTypes = new Set([
		'image/png',
		'image/jpeg',
		'image/webp',
		'image/gif',
	]);
	let { value = $bindable() }: Props = $props();
	let error = $state('');
	let dragging = $state(false);
	let input: HTMLInputElement;

	async function selectImage(event: Event): Promise<void> {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		await loadImage(file);
	}

	async function loadImage(file: File): Promise<void> {
		if (!imageTypes.has(file.type)) {
			error = 'Choose a PNG, JPEG, WebP or GIF image.';
			input.value = '';
			return;
		}
		if (file.size > 2 * 1024 * 1024) {
			error = 'The icon image must be smaller than 2 MB.';
			input.value = '';
			return;
		}
		error = '';
		try {
			value = await normalizedIcon(file);
		} catch (cause) {
			error =
				cause instanceof Error
					? cause.message
					: 'Unable to process icon image.';
			input.value = '';
		}
	}

	function dragOver(event: DragEvent): void {
		event.preventDefault();
		dragging = true;
	}

	function dropImage(event: DragEvent): void {
		event.preventDefault();
		dragging = false;
		const file = event.dataTransfer?.files[0];
		if (file) void loadImage(file);
	}

	function clear(): void {
		value = undefined;
		error = '';
		input.value = '';
	}

	async function normalizedIcon(file: File): Promise<string> {
		const image = await createImageBitmap(file);
		const scale = Math.min(1, 128 / image.width, 128 / image.height);
		const canvas = document.createElement('canvas');
		canvas.width = Math.max(1, Math.round(image.width * scale));
		canvas.height = Math.max(1, Math.round(image.height * scale));
		const context = canvas.getContext('2d');
		if (!context) throw new Error('Unable to prepare the icon image.');
		context.drawImage(image, 0, 0, canvas.width, canvas.height);
		image.close();
		for (const quality of [0.84, 0.68, 0.5]) {
			const blob = await canvasBlob(canvas, quality);
			if (blob.size <= 120 * 1024) return dataUrl(blob);
		}
		throw new Error('The icon image is too complex to store.');
	}

	function canvasBlob(
		canvas: HTMLCanvasElement,
		quality: number,
	): Promise<Blob> {
		return new Promise((resolve, reject) =>
			canvas.toBlob(
				(blob) =>
					blob
						? resolve(blob)
						: reject(new Error('Unable to encode the icon image.')),
				'image/webp',
				quality,
			),
		);
	}

	function dataUrl(file: Blob): Promise<string> {
		return new Promise((resolve, reject) => {
			const reader = new FileReader();
			reader.addEventListener('load', () => {
				if (typeof reader.result === 'string') resolve(reader.result);
				else reject(new Error('Icon image did not produce a data URL.'));
			});
			reader.addEventListener('error', () =>
				reject(reader.error ?? new Error('Unable to read icon image.')),
			);
			reader.readAsDataURL(file);
		});
	}
</script>

<div class="icon-image-control">
	<label
		class:dragging
		class="icon-image-field"
		aria-label={value ? 'Replace icon image' : 'Choose icon image'}
		title={value ? 'Replace icon image' : 'Choose icon image'}
		ondragover={dragOver}
		ondragleave={() => (dragging = false)}
		ondrop={dropImage}
	>
		<input
			bind:this={input}
			type="file"
			accept="image/png,image/jpeg,image/webp,image/gif"
			onchange={selectImage}
		/>
		{#if value}
			<img src={value} alt="Selected icon preview" />
		{:else}
			<ImagePlus size={19} aria-hidden="true" />
		{/if}
	</label>
	{#if value}
		<button
			type="button"
			class="clear-icon"
			onclick={clear}
			aria-label="Remove icon image"
			title="Remove icon"
		>
			<X size={12} />
		</button>
	{/if}
</div>
{#if error}<p class="icon-error" role="alert">{error}</p>{/if}

<style>
	.icon-image-control {
		position: relative;
		width: max-content;
		margin-top: 1rem;
	}

	.icon-image-field {
		display: grid;
		width: 3.25rem;
		height: 3.25rem;
		box-sizing: border-box;
		overflow: hidden;
		place-items: center;
		border: 1px dashed #8aa9ce;
		border-radius: 0.8rem;
		background: rgb(255 255 255 / 0.65);
		color: var(--site-link-fg);
		cursor: pointer;
		padding: 0.28rem;
		transition:
			background 120ms,
			border-color 120ms,
			box-shadow 120ms;
	}

	.icon-image-field:hover,
	.icon-image-field.dragging {
		border-color: var(--arcadia-focus);
		background: white;
		box-shadow: 0 0 0 3px rgb(37 134 232 / 0.1);
	}

	.icon-image-field input {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		clip-path: inset(50%);
	}

	.icon-image-field img {
		width: 100%;
		height: 100%;
		border-radius: 0.58rem;
		object-fit: cover;
	}

	.icon-image-field :global(svg) {
		pointer-events: none;
	}

	.clear-icon {
		position: absolute;
		top: -0.38rem;
		right: -0.38rem;
		display: grid;
		width: 1.35rem;
		height: 1.35rem;
		place-items: center;
		border: 1px solid var(--arcadia-line);
		border-radius: 50%;
		background: white;
		color: var(--arcadia-muted);
		cursor: pointer;
		padding: 0;
	}

	.icon-error {
		color: #a52a37;
		font-size: 0.7rem;
		margin: 0.35rem 0 0 0.1rem;
	}
</style>

<script lang="ts"></script>

<span class="spinner" aria-hidden="true"></span>

<style>
	.spinner {
		display: inline-block;
		width: 10px;
		height: 10px;
		border: 1.5px solid var(--line);
		border-top-color: var(--color-primary);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>

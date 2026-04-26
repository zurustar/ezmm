export class PreviewStore {
	isPlaying = $state<boolean>(false);
	currentTime = $state<number>(0);       // seconds, cumulative across project
	totalDuration = $state<number>(0);     // seconds, selected entry's total scene time
	audioContextReady = $state<boolean>(false);

	play() { this.isPlaying = true; }
	pause() { this.isPlaying = false; }
	seek(time: number) { this.currentTime = time; }
	resumeAudioContext() { this.audioContextReady = true; }
	setTotalDuration(duration: number) { this.totalDuration = duration; }
}

export const previewStore = new PreviewStore();

<script lang="ts">
  import { Figure, Image } from 'sveltestrap';

  let clientWidth: number;
</script>

<div bind:clientWidth>
  <Figure>
    <Image src="./placeholder.png" width={clientWidth} />
  </Figure>
</div>

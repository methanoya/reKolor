<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Row, Col } from 'sveltestrap';
  import type { PickingStripeType } from './types';
  import { pickingStripes } from './store';
  import { STRIPS_COLUMNS } from './constants';
  import PickingStripeComponent from './PickingStripeComponent.svelte';

  let stripes: PickingStripeType[] = [];
  onDestroy(
    pickingStripes.subscribe((value: PickingStripeType[]) => {
      // console.log('value', value);
      stripes = value;
    }),
  );

  $: console.log('stripes:', stripes);
</script>

<Row>
  <p>&nbsp;</p>
</Row>

<!--<Row>-->
<!--  {#each stripes as stripe}-->
<!--    <Col>-->
<!--      <PickingStripeComponent {stripe} />-->
<!--    </Col>-->
<!--  {/each}-->
<!--</Row>-->

{#each Array(Math.ceil(stripes.length / STRIPS_COLUMNS)) as _, row_index}
  <Row>
    {#each Array(STRIPS_COLUMNS) as _, column_index}
      <Col>
        {#if stripes[row_index * STRIPS_COLUMNS + column_index]}
          <PickingStripeComponent
            stripe={stripes[row_index * STRIPS_COLUMNS + column_index]}
          />
        {/if}
      </Col>
    {/each}
  </Row>
  <Row>
    <p>&nbsp;</p>
  </Row>
{/each}

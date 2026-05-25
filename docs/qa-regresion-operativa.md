# QA de regresion operativa Ferre-POS

Esta checklist se ejecuta antes de liberar cambios que toquen ventas, caja, inventario, facturacion, sincronizacion o permisos.

## Puerta tecnica

Ejecutar desde la raiz del proyecto:

```bash
npm run qa
```

Debe terminar sin errores. El comando valida TypeScript, compila Rust con `cargo check` y genera el build productivo de Vite.

## Datos base de prueba

- Crear dos sucursales activas.
- Crear un usuario `SUPERADMIN`, un `ADMIN` por sucursal y un `USUARIO` por sucursal.
- Crear proveedor, marca, categoria y unidad.
- Crear al menos dos productos:
  - Producto A con stock suficiente en Sucursal 1.
  - Producto B con stock minimo configurado para alertas.
- Crear un cliente con datos fiscales validos y limite de credito.
- Configurar emisor fiscal y codigo postal de sucursal.

## Caja

- Abrir caja con monto inicial mayor a cero.
- Intentar abrir una segunda caja en la misma sucursal: debe bloquearse.
- Registrar ingreso y egreso manual: deben afectar el monto esperado.
- Intentar cerrar caja con monto fisico cero cuando hay efectivo esperado: debe bloquearse.
- Cerrar caja con monto fisico valido: debe guardar diferencia y marcar sincronizacion pendiente.
- Intentar vender o registrar movimiento con caja cerrada: debe bloquearse.

## Ventas

- Buscar producto por codigo de barras: debe resolver exacto antes que coincidencias parciales.
- Agregar producto repetido al carrito: debe consolidar o descontar stock por impacto total.
- Cobrar en efectivo, tarjeta y transferencia: debe guardar venta completada y movimiento de caja cuando aplique.
- Intentar vender mas stock del disponible: debe bloquearse en backend.
- Venta a credito: debe validar limite disponible y guardar como credito sin romper caja.
- Promocion activa: debe mostrar precio original tachado, precio descontado y ahorro total.
- Promocion no debe permitir precio final menor que costo.

## Cancelaciones

- Cancelar venta completada con caja cerrada: debe bloquearse.
- Cancelar venta con caja abierta y autorizacion valida: debe devolver stock a la sucursal correcta.
- La cancelacion debe registrar movimiento `DEVOLUCION EN VENTA #ID`.
- Intentar cancelar venta con CFDI timbrado: debe bloquearse.
- Verificar que usuario autorizador, motivo y fecha queden registrados.

## Inventario, compras y mermas

- Registrar compra: debe aumentar stock en la sucursal seleccionada.
- Registrar merma: debe descontar stock de forma inmediata y no permitir negativos.
- Editar costo/precio/stock por sucursal: debe marcar `sincronizado = 0`.
- Producto eliminado logicamente no debe aparecer en busquedas ni inventario operativo.

## Traspasos

- Crear traspaso desde Sucursal 1 a Sucursal 2: debe descontar origen y quedar `EN_TRANSITO`.
- Si destino no tenia inventario del producto, al recibir debe crear la fila de inventario destino.
- ADMIN solo puede recibir traspasos enviados a su sucursal.
- SUPERADMIN puede recibir cualquier traspaso.
- Interrumpir recepcion a mitad de flujo no debe dejar stock duplicado ni perdido.

## Facturacion

- Generar payload CFDI desde venta completada: debe usar precios netos y desglosar IVA incluido.
- Venta contado: `MetodoPago = PUE` y forma de pago real.
- Venta credito: `MetodoPago = PPD` y `FormaPago = 99`.
- Producto sin clave SAT de 8 caracteres o unidad SAT invalida debe bloquear facturacion.
- Timbrar con UUID invalido o duplicado debe bloquearse.

## Sincronizacion

- Crear/editar productos, clientes, ventas, caja y facturas: deben quedar con `sincronizado = 0`.
- Worker debe subir lotes sin congelar UI.
- Desconectar internet: POS debe operar offline y reintentar despues.
- Borrado logico local debe subir `eliminado = 1`.
- Pull desde nube debe aplicar cambios nuevos y ocultar registros eliminados.
- Restaurar desde nube debe sobrescribir local solo con confirmacion del usuario.
- Subir respaldo a nube debe advertir que reemplazara datos remotos.

## Permisos

- ADMIN solo ve ventas, movimientos, caja, facturas e inventario de su sucursal.
- ADMIN no puede crear, editar ni eliminar ADMIN o SUPERADMIN.
- Ningun usuario puede editar o eliminar su propia cuenta.
- No se puede eliminar o degradar al ultimo SUPERADMIN activo.

## Resultado esperado

Si un punto falla, no liberar. Registrar modulo, paso, usuario usado, sucursal, hora y captura del error antes de corregir.

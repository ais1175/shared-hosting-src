อย่า run dev  นะครับ ขอเตือนไว้ก่อน  ถ้าจะใช้งานจริงๆ อย่ารัน dev ทั้ง nextjs และก็ rust 

และตอนนี้ มันออกแบบมารันใน windows ดูจาก folder script จะมี powershells file อยู่
ค่า default

อยู๋ใน

folder rust 
| main.rs

แล้วก็ ctrl + f แล้วพิม jwt

แล้วค้นหา รูปแบบcodeแบบนี้

            auth_config: Arc::new(AuthConfig {
                jwt_secret: "test-jwt-secret-12345678901234567890".to_owned(),
                refresh_secret: "test-refresh-secret-1234567890123".to_owned(),
ป้องกัน หากเริ่มใส่ env

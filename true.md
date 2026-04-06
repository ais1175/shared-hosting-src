📚 TrueMoney Voucher API - Node.js Documentation

🔗 API Endpoint

```
POST https://gift.truemoney.com/campaign/vouchers/{voucher_hash}/redeem
```

🛠️ Installation

```bash
npm install axios
# หรือ
yarn add axios
```

📋 Core Implementation

1. ฟังก์ชันหลักสำหรับเรียกใช้ API

```javascript
const axios = require('axios');

/**
 * รับซองอั่งเปา TrueMoney
 * @param {string} voucherHash - รหัสซองอั่งเปา
 * @param {string} phoneNumber - หมายเลขโทรศัพท์รับซอง
 * @returns {Promise<Object>} ผลลัพธ์
 */
async function redeemTrueMoneyVoucher(voucherHash, phoneNumber) {
    const url = `https://gift.truemoney.com/campaign/vouchers/${voucherHash}/redeem`;
    
    const postData = {
        mobile: phoneNumber,
        voucher_hash: voucherHash
    };
    
    const config = {
        headers: {
            'Content-Type': 'application/json',
            'Accept': 'application/json',
            'User-Agent': 'Mozilla/5.0',
            'Origin': 'https://gift.truemoney.com',
            'Referer': 'https://gift.truemoney.com/',
            'Accept-Language': 'th,en;q=0.9',
            'Connection': 'keep-alive'
        },
        timeout: 10000
    };
    
    try {
        const response = await axios.post(url, postData, config);
        return parseApiResponse(response.data);
    } catch (error) {
        if (error.response) {
            // API responded with error status
            return parseApiResponse(error.response.data);
        } else if (error.request) {
            // No response received
            return {
                success: false,
                message: `Connection error: ${error.message}`,
                amount: 0
            };
        } else {
            // Other errors
            return {
                success: false,
                message: `Error: ${error.message}`,
                amount: 0
            };
        }
    }
}

/**
 * ประมวลผล response จาก API
 * @param {Object} data - ข้อมูลจาก API
 * @returns {Object} ผลลัพธ์ที่ประมวลผลแล้ว
 */
function parseApiResponse(data) {
    if (!data.status || !data.status.code) {
        return {
            success: false,
            message: 'Invalid API response',
            amount: 0
        };
    }
    
    if (data.status.code === 'SUCCESS') {
        const amount = parseFloat(data.data?.voucher?.redeemed_amount_baht) || 0;
        return {
            success: true,
            message: `รับซองอั่งเปาสำเร็จ ${amount} บาท`,
            amount: amount,
            raw_response: data
        };
    } else {
        const errorMessage = getErrorMessage(data.status.code);
        return {
            success: false,
            message: errorMessage,
            amount: 0,
            error_code: data.status.code
        };
    }
}

/**
 * แปลงรหัสข้อผิดพลาดเป็นข้อความ
 * @param {string} code - รหัสข้อผิดพลาด
 * @returns {string} ข้อความข้อผิดพลาด
 */
function getErrorMessage(code) {
    const errorMessages = {
        'MISSING_RECEIVER_PHONE_NUMBER': 'กรุณากรอกเบอร์รับซองอั่งเปา',
        'MISSING_GIFT_CODE_OR_URL': 'กรุณากรอกลิงก์ซองอั่งเปาหรือเบอร์โทรศัพท์ให้ถูกต้อง',
        'INVALID_RECEIVER_PHONE_NUMBER_FORMAT': 'รูปแบบเบอร์ไม่ถูกต้อง',
        'INVALID_GIFT_CODE_OR_URL_FORMAT': 'รูปแบบลิงก์ซองอั่งเปาไม่ถูกต้อง',
        'CANNOT_GET_OWN_VOUCHER': 'ไม่สามารถรับซองอั่งเปาของตัวเองได้',
        'VOUCHER_OUT_OF_STOCK': 'ซองอั่งเปานี้ถูกรับไปหมดแล้ว',
        'VOUCHER_EXPIRED': 'ซองอั่งเปาหมดอายุแล้ว',
        'UNEXPECTED_ERROR': 'เกิดข้อผิดพลาดในการรับซองอั่งเปา'
    };
    
    return errorMessages[code] || `เกิดข้อผิดพลาด: ${code}`;
}

/**
 * สกัดรหัสซองจากลิงก์
 * @param {string} link - ลิงก์ซองอั่งเปา
 * @returns {string|null} รหัสซอง
 */
function extractVoucherHash(link) {
    const regex = /gift\.truemoney\.com\/campaign\/\?v=([A-Za-z0-9]+)/;
    const match = link.match(regex);
    return match ? match[1] : null;
}

module.exports = {
    redeemTrueMoneyVoucher,
    extractVoucherHash,
    getErrorMessage
};
```

2. ไฟล์ใช้งานหลัก (main.js)

```javascript
const { redeemTrueMoneyVoucher, extractVoucherHash } = require('./truemoney-api');

/**
 * ตัวอย่างการใช้งานแบบพื้นฐาน
 */
async function main() {
    const voucherLink = "https://gift.truemoney.com/campaign/?v=ABC123XYZ";
    const phoneNumber = "0899999999";
    
    // สกัดรหัสซองจากลิงก์
    const voucherHash = extractVoucherHash(voucherLink);
    
    if (!voucherHash) {
        console.log("❌ ลิงก์ซองอั่งเปาไม่ถูกต้อง");
        return;
    }
    
    console.log(`🔗 Voucher Hash: ${voucherHash}`);
    console.log(`📱 Phone: ${phoneNumber}`);
    console.log("🔄 กำลังรับซองอั่งเปา...");
    
    // เรียกใช้ API
    const result = await redeemTrueMoneyVoucher(voucherHash, phoneNumber);
    
    // แสดงผล
    if (result.success) {
        console.log("✅ สำเร็จ:", result.message);
        console.log("💰 ได้รับเงิน:", result.amount, "บาท");
    } else {
        console.log("❌ ล้มเหลว:", result.message);
        if (result.error_code) {
            console.log("📋 รหัสข้อผิดพลาด:", result.error_code);
        }
    }
}

// เรียกใช้งาน
main().catch(console.error);
```

3. ใช้งานกับ Express.js Server

```javascript
const express = require('express');
const { redeemTrueMoneyVoucher, extractVoucherHash } = require('./truemoney-api');

const app = express();
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

/**
 * API Endpoint สำหรับรับซองอั่งเปา
 * POST /api/redeem
 */
app.post('/api/redeem', async (req, res) => {
    try {
        const { username, voucher_link, phone_number = "0899999999" } = req.body;
        
        // Validate input
        if (!username || !voucher_link) {
            return res.json({
                success: false,
                message: 'กรุณากรอก username และลิงก์ซองอั่งเปา'
            });
        }
        
        // Extract voucher hash
        const voucherHash = extractVoucherHash(voucher_link);
        if (!voucherHash) {
            return res.json({
                success: false,
                message: 'ลิงก์ซองอั่งเปาไม่ถูกต้อง'
            });
        }
        
        console.log(`🎯 User ${username} is redeeming voucher: ${voucherHash}`);
        
        // Call TrueMoney API
        const apiResult = await redeemTrueMoneyVoucher(voucherHash, phone_number);
        
        // If successful, update user credit in database
        if (apiResult.success && apiResult.amount > 0) {
            const creditUpdated = await updateUserCredit(username, apiResult.amount);
            apiResult.credit_updated = creditUpdated;
        }
        
        res.json(apiResult);
        
    } catch (error) {
        console.error('Server error:', error);
        res.json({
            success: false,
            message: 'Server error: ' + error.message
        });
    }
});

/**
 * อัพเดทเครดิตผู้ใช้ใน database
 */
async function updateUserCredit(username, amount) {
    // ตัวอย่างการเชื่อมต่อ database
    // const result = await db.collection('users').updateOne(
    //     { username: username },
    //     { $inc: { credit: amount } }
    // );
    // return result.modifiedCount > 0;
    
    console.log(`💳 Updating credit for ${username}: +${amount} THB`);
    return true; // Simulate success
}

/**
 * Health check endpoint
 */
app.get('/health', (req, res) => {
    res.json({ status: 'OK', timestamp: new Date().toISOString() });
});

// Start server
const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
    console.log(`🚀 Server running on port ${PORT}`);
    console.log(`📚 API Documentation: http://localhost:${PORT}/api/redeem`);
});
```

4. ใช้งานแบบ Advanced with Error Handling

```javascript
const { redeemTrueMoneyVoucher, extractVoucherHash } = require('./truemoney-api');

class VoucherService {
    constructor(defaultPhone = '0899999999') {
        this.defaultPhone = defaultPhone;
    }
    
    /**
     * รับซองอั่งเปาพร้อมการจัดการข้อผิดพลาดแบบครบวงจร
     */
    async redeemVoucher(voucherLink, options = {}) {
        const {
            phoneNumber = this.defaultPhone,
            username = null,
            maxRetries = 1
        } = options;
        
        try {
            // Validate input
            if (!voucherLink) {
                throw new Error('Voucher link is required');
            }
            
            // Extract voucher hash
            const voucherHash = extractVoucherHash(voucherLink);
            if (!voucherHash) {
                return {
                    success: false,
                    message: 'ลิงก์ซองอั่งเปาไม่ถูกต้อง',
                    amount: 0
                };
            }
            
            // Call API with retry logic
            let result;
            for (let attempt = 1; attempt <= maxRetries; attempt++) {
                console.log(`🔄 Attempt ${attempt} for voucher: ${voucherHash}`);
                
                result = await redeemTrueMoneyVoucher(voucherHash, phoneNumber);
                
                if (result.success || attempt === maxRetries) {
                    break;
                }
                
                // Wait before retry
                await this.sleep(1000);
            }
            
            // Log result
            this.logResult(username, voucherHash, result);
            
            return result;
            
        } catch (error) {
            console.error('Voucher redemption error:', error);
            return {
                success: false,
                message: `System error: ${error.message}`,
                amount: 0
            };
        }
    }
    
    /**
     * รับหลายซองพร้อมกัน
     */
    async redeemMultipleVouchers(voucherLinks, phoneNumber) {
        const promises = voucherLinks.map(link => 
            this.redeemVoucher(link, { phoneNumber })
        );
        
        const results = await Promise.allSettled(promises);
        
        return results.map((result, index) => ({
            voucher: voucherLinks[index],
            ...(result.status === 'fulfilled' ? result.value : {
                success: false,
                message: 'Promise rejected',
                amount: 0
            })
        }));
    }
    
    /**
     * Utility functions
     */
    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
    
    logResult(username, voucherHash, result) {
        const logEntry = {
            timestamp: new Date().toISOString(),
            username: username,
            voucher_hash: voucherHash,
            success: result.success,
            amount: result.amount,
            message: result.message
        };
        
        console.log('📝 Voucher Redemption Log:', JSON.stringify(logEntry));
    }
}

// ตัวอย่างการใช้งาน Class
async function exampleUsage() {
    const voucherService = new VoucherService('0899999999');
    
    // รับซองเดียว
    const result = await voucherService.redeemVoucher(
        'https://gift.truemoney.com/campaign/?v=ABC123XYZ',
        { username: 'john_doe' }
    );
    
    console.log('Single voucher result:', result);
    
    // รับหลายซอง
    const vouchers = [
        'https://gift.truemoney.com/campaign/?v=ABC123XYZ',
        'https://gift.truemoney.com/campaign/?v=DEF456UVW',
        'https://gift.truemoney.com/campaign/?v=GHI789RST'
    ];
    
    const multipleResults = await voucherService.redeemMultipleVouchers(
        vouchers, 
        '0899999999'
    );
    
    console.log('Multiple vouchers results:');
    multipleResults.forEach((result, index) => {
        console.log(`${index + 1}. ${result.voucher} - ${result.success ? '✅' : '❌'} ${result.message}`);
    });
}

// เรียกใช้งานตัวอย่าง
// exampleUsage();
```

5. Package.json Example

```json
{
  "name": "truemoney-voucher-api",
  "version": "1.0.0",
  "description": "TrueMoney Voucher Redemption API",
  "main": "index.js",
  "scripts": {
    "start": "node server.js",
    "dev": "nodemon server.js",
    "test": "node test.js"
  },
  "dependencies": {
    "axios": "^1.6.0",
    "express": "^4.18.0"
  },
  "devDependencies": {
    "nodemon": "^3.0.0"
  },
  "keywords": [
    "truemoney",
    "voucher",
    "api",
    "thailand"
  ]
}
```

🚀 Quick Start

1. ติดตั้ง dependencies:

```bash
npm install
```

1. สร้างไฟล์ truemoney-api.js ด้วยโค้ดด้านบน
2. รันเซิร์ฟเวอร์:

```bash
node server.js
```

1. ทดสอบ API:

```bash
curl -X POST http://localhost:3000/api/redeem \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "voucher_link": "https://gift.truemoney.com/campaign/?v=ABC123XYZ"
  }'
```

📝 Response Format

สำเร็จ:

```json
{
  "success": true,
  "message": "รับซองอั่งเปาสำเร็จ 50 บาท",
  "amount": 50,
  "credit_updated": true
}
```

ล้มเหลว:

```json
{
  "success": false,
  "message": "ซองอั่งเปานี้ถูกรับไปหมดแล้ว",
  "amount": 0,
  "error_code": "VOUCHER_OUT_OF_STOCK"
}
```

พร้อมใช้งานแล้ว! 🎯
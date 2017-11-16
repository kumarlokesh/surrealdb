// Copyright © 2016 Abcum Ltd
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package conv

import (
	"fmt"
	"strconv"
	"time"

	"github.com/abcum/surreal/sql"

	"github.com/asaskevich/govalidator"
)

func toNumber(str string) (float64, error) {
	val, err := strconv.ParseFloat(str, 64)
	if err != nil {
		val = 0.0
		err = fmt.Errorf("Expected a number, but found '%v'", str)

	}
	return float64(int64(val)), err
}

func toDouble(str string) (float64, error) {
	val, err := strconv.ParseFloat(str, 64)
	if err != nil {
		val = 0.0
		err = fmt.Errorf("Expected a number, but found '%v'", str)
	}
	return float64(val), err
}

func toBoolean(str string) (bool, error) {
	val, err := strconv.ParseBool(str)
	if err != nil {
		val = false
		err = fmt.Errorf("Expected a boolean, but found '%v'", str)
	}
	return bool(val), err
}

// --------------------------------------------------

func ConvertTo(t, k string, obj interface{}) (val interface{}, err error) {
	switch t {
	default:
		return obj, nil
	case "url":
		return ConvertToUrl(obj)
	case "uuid":
		return ConvertToUuid(obj)
	case "color":
		return ConvertToColor(obj)
	case "email":
		return ConvertToEmail(obj)
	case "phone":
		return ConvertToPhone(obj)
	case "array":
		return ConvertToArray(obj)
	case "object":
		return ConvertToObject(obj)
	case "domain":
		return ConvertToDomain(obj)
	case "base64":
		return ConvertToBase64(obj)
	case "string":
		return ConvertToString(obj)
	case "number":
		return ConvertToNumber(obj)
	case "double":
		return ConvertToDouble(obj)
	case "boolean":
		return ConvertToBoolean(obj)
	case "datetime":
		return ConvertToDatetime(obj)
	case "latitude":
		return ConvertToLatitude(obj)
	case "longitude":
		return ConvertToLongitude(obj)
	case "record":
		return ConvertToRecord(obj, k)
	}
}

func ConvertToUrl(obj interface{}) (val string, err error) {
	val = fmt.Sprintf("%v", obj)
	if !govalidator.IsURL(val) {
		err = fmt.Errorf("Expected a URL, but found '%v'", obj)
	}
	return
}

func ConvertToUuid(obj interface{}) (val string, err error) {
	val = fmt.Sprintf("%v", obj)
	if !govalidator.IsUUID(val) {
		err = fmt.Errorf("Expected a UUID, but found '%v'", obj)
	}
	return
}

func ConvertToEmail(obj interface{}) (val string, err error) {
	val = fmt.Sprintf("%v", obj)
	if !govalidator.IsEmail(val) {
		err = fmt.Errorf("Expected an email address, but found '%v'", obj)
	}
	return govalidator.NormalizeEmail(val)
}

func ConvertToPhone(obj interface{}) (val string, err error) {
	val = fmt.Sprintf("%v", obj)
	if !govalidator.Matches(val, `^[\s\d\+\-\(\)]+$`) {
		err = fmt.Errorf("Expected a phone number, but found '%v'", obj)
	}
	return
}

func ConvertToColor(obj interface{}) (val string, err error) {
	val = fmt.Sprintf("%v", obj)
	if !govalidator.IsHexcolor(val) && !govalidator.IsRGBcolor(val) {
		err = fmt.Errorf("Expected a HEX or RGB color, but found '%v'", obj)
	}
	return
}

func ConvertToArray(obj interface{}) (val []interface{}, err error) {
	if now, ok := obj.([]interface{}); ok {
		val = now
	} else {
		err = fmt.Errorf("Expected an array, but found '%v'", obj)
	}
	return
}

func ConvertToObject(obj interface{}) (val map[string]interface{}, err error) {
	if now, ok := obj.(map[string]interface{}); ok {
		val = now
	} else {
		err = fmt.Errorf("Expected an object, but found '%v'", obj)
	}
	return
}

func ConvertToDomain(obj interface{}) (val string, err error) {
	val = fmt.Sprintf("%v", obj)
	if !govalidator.IsDNSName(val) {
		err = fmt.Errorf("Expected a domain name, but found '%v'", obj)
	}
	return
}

func ConvertToBase64(obj interface{}) (val string, err error) {
	val = fmt.Sprintf("%v", obj)
	if !govalidator.IsBase64(val) {
		err = fmt.Errorf("Expected base64 data, but found '%v'", obj)
	}
	return
}

func ConvertToString(obj interface{}) (val string, err error) {
	switch now := obj.(type) {
	case string:
		return now, err
	case []interface{}, map[string]interface{}:
		return val, fmt.Errorf("Expected a string, but found '%v'", obj)
	default:
		return fmt.Sprintf("%v", obj), err
	}
}

func ConvertToNumber(obj interface{}) (val float64, err error) {
	switch now := obj.(type) {
	case int64:
		return float64(now), err
	case float64:
		return float64(now), err
	case string:
		return toNumber(now)
	default:
		return toNumber(fmt.Sprintf("%v", obj))
	}
}

func ConvertToDouble(obj interface{}) (val float64, err error) {
	switch now := obj.(type) {
	case int64:
		return float64(now), err
	case float64:
		return float64(now), err
	case string:
		return toDouble(now)
	default:
		return toDouble(fmt.Sprintf("%v", obj))
	}
}

func ConvertToBoolean(obj interface{}) (val bool, err error) {
	switch now := obj.(type) {
	case int64:
		return now > 0, err
	case float64:
		return now > 0, err
	case string:
		return toBoolean(now)
	default:
		return toBoolean(fmt.Sprintf("%v", obj))
	}
}

func ConvertToDatetime(obj interface{}) (val time.Time, err error) {
	if now, ok := obj.(time.Time); ok {
		val = now
	} else {
		err = fmt.Errorf("Expected a datetime, but found '%v'", obj)
	}
	return
}

func ConvertToLatitude(obj interface{}) (val float64, err error) {
	str := fmt.Sprintf("%v", obj)
	if !govalidator.IsLatitude(str) {
		err = fmt.Errorf("Expected a latitude value, but found '%v'", obj)
	}
	return govalidator.ToFloat(str)
}

func ConvertToLongitude(obj interface{}) (val float64, err error) {
	str := fmt.Sprintf("%v", obj)
	if !govalidator.IsLongitude(str) {
		err = fmt.Errorf("Expected a longitude value, but found '%v'", obj)
	}
	return govalidator.ToFloat(str)
}

func ConvertToRecord(obj interface{}, tb string) (val *sql.Thing, err error) {
	if now, ok := obj.(*sql.Thing); ok {
		switch tb {
		case now.TB:
			val = now
		case "":
			val = now
		default:
			err = fmt.Errorf("Expected a record of type '%s', but found '%v'", tb, obj)
		}
	} else {
		err = fmt.Errorf("Expected a record of type '%s', but found '%v'", tb, obj)
	}
	return
}
